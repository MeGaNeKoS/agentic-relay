use std::time::Duration;

use serde_json::Value;
use tokio::sync::oneshot;

use super::jsonrpc::CallError;

pub struct TurnStartedWaiter(pub(crate) oneshot::Receiver<()>);

impl TurnStartedWaiter {
    pub async fn wait(self, timeout: Duration) -> bool {
        matches!(tokio::time::timeout(timeout, self.0).await, Ok(Ok(())))
    }
}

#[async_trait::async_trait]
pub trait CodexApi: Send + Sync {
    async fn call(&self, method: &str, params: Value) -> Result<Value, CallError>;

    async fn expect_turn_started(&self, thread_id: &str) -> Result<TurnStartedWaiter, CallError>;
}
