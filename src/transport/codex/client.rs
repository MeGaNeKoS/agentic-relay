use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use serde_json::Value;
use tokio::sync::Mutex;

use super::api::{CodexApi, TurnStartedWaiter};
use super::daemon;
use super::jsonrpc::{CALL_TIMEOUT, CallError, RpcClient};

const CONNECT_TIMEOUT: Duration = CALL_TIMEOUT.saturating_add(Duration::from_secs(30));

pub struct CodexClient {
    client_name: String,
    slot: StdMutex<Option<Arc<RpcClient>>>,
    dial_lock: Mutex<()>,
}

impl CodexClient {
    pub fn new(client_name: &str) -> Arc<Self> {
        Arc::new(Self { client_name: client_name.to_string(), slot: StdMutex::new(None), dial_lock: Mutex::new(()) })
    }

    fn open_connection(&self) -> Option<Arc<RpcClient>> {
        self.slot.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone().filter(|rpc| !rpc.is_closed())
    }

    async fn connection(&self) -> Result<Arc<RpcClient>, String> {
        if let Some(rpc) = self.open_connection() {
            return Ok(rpc);
        }
        let _dialing = self.dial_lock.lock().await;
        if let Some(rpc) = self.open_connection() {
            return Ok(rpc);
        }
        let rpc = tokio::time::timeout(CONNECT_TIMEOUT, daemon::connect(&self.client_name))
            .await
            .map_err(|_| format!("connecting to the codex daemon took longer than {CONNECT_TIMEOUT:?}"))?
            .map_err(|e| format!("the codex daemon is unreachable: {e:#}"))?;
        *self.slot.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(rpc.clone());
        Ok(rpc)
    }
}

#[async_trait::async_trait]
impl CodexApi for CodexClient {
    async fn call(&self, method: &str, params: Value) -> Result<Value, CallError> {
        let rpc = self.connection().await.map_err(CallError::NotSent)?;
        rpc.call(method, params).await
    }

    async fn expect_turn_started(&self, thread_id: &str) -> Result<TurnStartedWaiter, CallError> {
        let rpc = self.connection().await.map_err(CallError::NotSent)?;
        Ok(rpc.expect_turn_started(thread_id))
    }
}
