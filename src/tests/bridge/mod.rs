mod create_failures;
mod create_naming_timeout;
mod create_naming_wait;
mod create_order;
mod deliver_failures;
mod deliver_table;
mod lookup_found;
mod lookup_refused;
mod relative_cwd_refused;
mod send_delivered;
mod send_refused;
mod serve_wire;

use std::collections::{HashMap, VecDeque};
use std::time::Duration;
use std::sync::{Arc, Mutex};

use anyhow::{Result, bail};
use serde_json::{Value, json};

use super::Bridge;
use crate::identity::CallerEnvironment;
use crate::process::Ancestor;
use crate::protocol::Candidate;
use crate::transport::claude::SessionFile;
use crate::transport::codex::jsonrpc::RpcError;
use crate::transport::codex::{CallError, CodexApi, TurnStartedWaiter};

pub(super) const NAMING_WAIT: Duration = Duration::from_millis(300);
pub(super) const MESSAGE_ID: &str = "5b0f0a52-6c1c-4c8e-9a52-1d6e0d2f1f10";
pub(super) const CALLER_SESSION: &str = "caller-session";
pub(super) const CALLER_PID: u32 = 50;

#[derive(Default)]
pub(super) struct FakeCodex {
    script: Mutex<HashMap<String, VecDeque<Result<Value, CallError>>>>,
    calls: Mutex<Vec<(String, Value)>>,
    started: Mutex<HashMap<String, tokio::sync::oneshot::Sender<()>>>,
    hold_turn_started: std::sync::atomic::AtomicBool,
}

impl FakeCodex {
    pub fn on(&self, method: &str, result: Result<Value, CallError>) -> &Self {
        self.script.lock().unwrap().entry(method.to_string()).or_default().push_back(result);
        self
    }

    pub fn hold_turn_started(&self) -> &Self {
        self.hold_turn_started.store(true, std::sync::atomic::Ordering::SeqCst);
        self
    }

    pub fn announce_turn_started(&self, thread_id: &str) {
        if let Some(tx) = self.started.lock().unwrap().remove(thread_id) {
            let _ = tx.send(());
        }
    }

    pub fn calls(&self) -> Vec<(String, Value)> {
        self.calls.lock().unwrap().clone()
    }

    pub fn methods(&self) -> Vec<String> {
        self.calls().into_iter().map(|(method, _)| method).collect()
    }

    pub fn params_of(&self, method: &str) -> Vec<Value> {
        self.calls().into_iter().filter(|(m, _)| m == method).map(|(_, params)| params).collect()
    }
}

#[async_trait::async_trait]
impl CodexApi for FakeCodex {
    async fn call(&self, method: &str, params: Value) -> Result<Value, CallError> {
        self.calls.lock().unwrap().push((method.to_string(), params));
        let scripted = self.script.lock().unwrap().get_mut(method).and_then(VecDeque::pop_front);
        if method == "turn/start" && scripted.as_ref().is_some_and(Result::is_ok) && !self.hold_turn_started.load(std::sync::atomic::Ordering::SeqCst) {
            let thread_id = self.calls.lock().unwrap().last().and_then(|(_, p)| p["threadId"].as_str().map(str::to_string));
            if let Some(thread_id) = thread_id {
                self.announce_turn_started(&thread_id);
            }
        }
        scripted.unwrap_or_else(|| Err(CallError::Rejected(RpcError { code: None, message: format!("unscripted call {method}"), data: None })))
    }

    async fn expect_turn_started(&self, thread_id: &str) -> Result<TurnStartedWaiter, CallError> {
        self.calls.lock().unwrap().push(("expect_turn_started".to_string(), json!({"threadId": thread_id})));
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.started.lock().unwrap().insert(thread_id.to_string(), tx);
        Ok(TurnStartedWaiter(rx))
    }
}

pub(super) fn not_steerable() -> CallError {
    CallError::Rejected(RpcError {
        code: Some(-32600),
        message: "cannot steer a review turn".to_string(),
        data: Some(json!({"codexErrorInfo": {"activeTurnNotSteerable": {"turnKind": "review"}}})),
    })
}

pub(super) fn rejected(message: &str) -> CallError {
    CallError::Rejected(RpcError { code: Some(-32600), message: message.to_string(), data: None })
}

pub(super) fn lost(reason: &str) -> CallError {
    CallError::Lost(reason.to_string())
}

pub(super) fn resumed(status: Value, newest_turns: Value) -> Value {
    json!({"thread": {"id": "t1", "status": status, "turns": []}, "initialTurnsPage": {"data": newest_turns}})
}

pub(super) fn idle() -> Value {
    resumed(json!({"type": "idle"}), json!([]))
}

pub(super) fn active(turn_id: &str) -> Value {
    resumed(json!({"type": "active", "activeFlags": []}), json!([{"id": turn_id, "status": "inProgress", "items": []}]))
}

pub(super) fn listed(threads: &[(&str, Option<&str>, &str)]) -> Value {
    let data: Vec<Value> = threads
        .iter()
        .map(|(id, name, cwd)| match name {
            Some(name) => json!({"id": id, "name": name, "cwd": cwd}),
            None => json!({"id": id, "cwd": cwd}),
        })
        .collect();
    json!({"data": data, "nextCursor": null})
}

pub(super) struct ClaudeCallerEnvironment;

impl CallerEnvironment for ClaudeCallerEnvironment {
    fn ancestry(&self, _pid: u32) -> Result<Vec<Ancestor>> {
        Ok(vec![Ancestor { pid: 1, start: 10 }, Ancestor { pid: CALLER_PID, start: 500 }])
    }

    fn claude_session(&self, pid: u32) -> Result<Option<SessionFile>> {
        Ok((pid == CALLER_PID).then(|| SessionFile {
            pid,
            session_id: CALLER_SESSION.to_string(),
            proc_start: 500,
            pid_domain: "win32:test".to_string(),
            messaging_socket_path: String::new(),
        }))
    }

    fn codex_daemon(&self) -> Result<Ancestor> {
        bail!("no daemon in this environment")
    }
}

pub(super) struct CodexCallerEnvironment;

impl CallerEnvironment for CodexCallerEnvironment {
    fn ancestry(&self, _pid: u32) -> Result<Vec<Ancestor>> {
        Ok(vec![Ancestor { pid: 1, start: 10 }, Ancestor { pid: 60, start: 600 }])
    }

    fn claude_session(&self, _pid: u32) -> Result<Option<SessionFile>> {
        Ok(None)
    }

    fn codex_daemon(&self) -> Result<Ancestor> {
        Ok(Ancestor { pid: 60, start: 600 })
    }
}

pub(super) fn claude_caller() -> Vec<Candidate> {
    vec![Candidate::Claude { pid: CALLER_PID, session_id: CALLER_SESSION.to_string() }]
}

pub(super) fn bridge(codex: &Arc<FakeCodex>, env: Arc<dyn CallerEnvironment>) -> Bridge {
    Bridge::new(codex.clone(), env, std::env::temp_dir().join("relay-test-no-sessions"))
}

pub(super) fn bridge_for_claude_caller(codex: &Arc<FakeCodex>) -> Bridge {
    bridge(codex, Arc::new(ClaudeCallerEnvironment))
}
