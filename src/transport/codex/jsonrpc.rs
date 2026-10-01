use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex as StdMutex};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::time::Duration;

use anyhow::{Result, anyhow};
use serde_json::Value;
use tokio::sync::{Mutex, oneshot};

use super::{TurnStartedWaiter, WsConn, WsReadHalf, WsWriteHalf};

pub const CALL_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, serde::Deserialize)]
struct RpcMessage {
    id: Option<Value>,
    method: Option<String>,
    #[serde(default)]
    params: Option<Value>,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpcError {
    pub code: Option<i64>,
    pub message: String,
    pub data: Option<Value>,
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.code {
            Some(code) => write!(f, "{} (code {code})", self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for RpcError {}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CallError {
    #[error("{0}")]
    NotSent(String),
    #[error("{0}")]
    Rejected(RpcError),
    #[error("{0}")]
    Lost(String),
}

fn parse_rpc_error(v: &Value) -> RpcError {
    let code = v.get("code").and_then(Value::as_i64);
    let message = match v.get("message").and_then(Value::as_str) {
        Some(m) => m.to_string(),
        None => format!("codex returned a malformed error object with no message: {v}"),
    };
    RpcError { code, message, data: v.get("data").cloned() }
}

type Waiter = oneshot::Sender<Result<Value, CallError>>;

struct Pending {
    closed: bool,
    calls: HashMap<i64, Waiter>,
}

pub struct RpcClient {
    call_timeout: Duration,
    writer: Mutex<WsWriteHalf>,
    next_id: AtomicI64,
    pending: Mutex<Pending>,
    closed: AtomicBool,
    turn_started: StdMutex<HashMap<String, oneshot::Sender<()>>>,
}

impl RpcClient {
    pub fn new(conn: WsConn, call_timeout: Duration) -> (Arc<Self>, WsReadHalf) {
        let client = Arc::new(Self {
            call_timeout,
            writer: Mutex::new(conn.write),
            next_id: AtomicI64::new(0),
            pending: Mutex::new(Pending { closed: false, calls: HashMap::new() }),
            closed: AtomicBool::new(false),
            turn_started: StdMutex::new(HashMap::new()),
        });
        (client, conn.read)
    }

    pub fn expect_turn_started(&self, thread_id: &str) -> TurnStartedWaiter {
        let (tx, rx) = oneshot::channel();
        let mut waiters = self.turn_started.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        waiters.retain(|_, tx| !tx.is_closed());
        waiters.insert(thread_id.to_string(), tx);
        TurnStartedWaiter(rx)
    }

    fn announce_turn_started(&self, params: Option<&Value>) {
        let Some(thread_id) = params.and_then(|p| p.get("threadId")).and_then(Value::as_str) else { return };
        let waiter = self.turn_started.lock().unwrap_or_else(std::sync::PoisonError::into_inner).remove(thread_id);
        if let Some(tx) = waiter {
            let _ = tx.send(());
        }
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    pub async fn call(&self, method: &str, params: Value) -> Result<Value, CallError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req = serde_json::json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        let body = serde_json::to_vec(&req).map_err(|e| CallError::NotSent(format!("encoding {method}: {e}")))?;

        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().await;
            if pending.closed {
                return Err(CallError::NotSent(format!("connection to the codex daemon was lost before {method} was sent")));
            }
            pending.calls.insert(id, tx);
        }

        if let Err(e) = self.writer.lock().await.write_frame(&body) {
            self.pending.lock().await.calls.remove(&id);
            return Err(CallError::NotSent(format!("{method} could not be written to the codex daemon: {e}")));
        }

        match tokio::time::timeout(self.call_timeout, rx).await {
            Ok(Ok(answer)) => answer,
            Ok(Err(_)) => Err(CallError::Lost(format!("connection closed before {method} answered"))),
            Err(_) => {
                self.pending.lock().await.calls.remove(&id);
                Err(CallError::Lost(format!("{method} got no answer within {:?}", self.call_timeout)))
            }
        }
    }

    pub async fn notify(&self, method: &str, params: Value) -> Result<()> {
        let body = serde_json::to_vec(&serde_json::json!({"jsonrpc": "2.0", "method": method, "params": params}))?;
        self.writer.lock().await.write_frame(&body)
    }

    pub async fn fail_pending(&self, reason: &str) {
        let mut pending = self.pending.lock().await;
        pending.closed = true;
        self.closed.store(true, Ordering::SeqCst);
        for (_, tx) in pending.calls.drain() {
            let _ = tx.send(Err(CallError::Lost(reason.to_string())));
        }
    }

    /// Reads frames until the connection ends. A frame carrying a `method`
    /// comes from the daemon: a request is an ask meant for whichever
    /// subscribed connection answers first, and relay never answers one, since
    /// its reply would decide an ask meant for someone else.
    pub async fn pump(self: Arc<Self>, mut reader: WsReadHalf) -> Result<()> {
        loop {
            let payload = reader.read_frame().await?;
            let msg: RpcMessage = serde_json::from_slice(&payload).map_err(|e| anyhow!("malformed frame from codex daemon: {e}"))?;
            if let Some(method) = msg.method {
                if msg.id.is_some() {
                    tracing::debug!("codex ask {method} left unanswered");
                } else if method == "turn/started" {
                    self.announce_turn_started(msg.params.as_ref());
                }
                continue;
            }
            let Some(id_value) = msg.id else { continue };
            let waiter = match id_value.as_i64() {
                Some(n) => self.pending.lock().await.calls.remove(&n),
                None => None,
            };
            let Some(tx) = waiter else {
                tracing::warn!("codex: response for unknown request id {id_value}");
                continue;
            };
            let answer = match (msg.error, msg.result) {
                (Some(e), _) => Err(CallError::Rejected(parse_rpc_error(&e))),
                (None, Some(v)) => Ok(v),
                (None, None) => Err(CallError::Rejected(RpcError {
                    code: None,
                    message: format!("response {id_value} has neither result nor error"),
                    data: None,
                })),
            };
            let _ = tx.send(answer);
        }
    }
}
