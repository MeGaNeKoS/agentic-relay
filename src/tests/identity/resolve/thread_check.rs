use std::sync::Mutex;

use serde_json::{Value, json};

use super::*;
use crate::identity::{Identity, resolve_caller};
use crate::transport::codex::jsonrpc::RpcError;
use crate::transport::codex::{CallError, CodexApi};

struct OneRead(Mutex<Option<Result<Value, CallError>>>);

#[async_trait::async_trait]
impl CodexApi for OneRead {
    async fn call(&self, method: &str, _params: Value) -> Result<Value, CallError> {
        assert_eq!(method, "thread/read");
        self.0.lock().unwrap().take().expect("one read only")
    }

    async fn expect_turn_started(&self, _thread_id: &str) -> Result<crate::transport::codex::TurnStartedWaiter, CallError> {
        unreachable!("identity checks never start turns")
    }
}

struct NoCalls;

#[async_trait::async_trait]
impl CodexApi for NoCalls {
    async fn call(&self, method: &str, _params: Value) -> Result<Value, CallError> {
        panic!("unexpected call {method}");
    }

    async fn expect_turn_started(&self, _thread_id: &str) -> Result<crate::transport::codex::TurnStartedWaiter, CallError> {
        unreachable!("identity checks never start turns")
    }
}

fn under_daemon() -> FakeEnvironment {
    FakeEnvironment { chain: vec![ancestor(1, 10), ancestor(60, 600)], sessions: vec![], daemon: Some(ancestor(60, 600)) }
}

#[tokio::test]
async fn codex_caller_is_accepted_when_the_thread_exists() {
    let codex_api = OneRead(Mutex::new(Some(Ok(json!({"thread": {}})))));
    let identity = resolve_caller(&[codex("t1")], 1, &under_daemon(), &codex_api).await.unwrap();
    assert_eq!(identity, Identity::Codex { thread_id: "t1".into() });
}

#[tokio::test]
async fn codex_caller_is_refused_when_the_thread_does_not_exist() {
    let missing = CallError::Rejected(RpcError { code: Some(-32600), message: "thread not found".into(), data: None });
    let codex_api = OneRead(Mutex::new(Some(Err(missing))));
    let refusal = resolve_caller(&[codex("t1")], 1, &under_daemon(), &codex_api).await.unwrap_err();
    assert!(refusal.0.contains("thread not found"), "{}", refusal.0);
}

#[tokio::test]
async fn codex_caller_is_refused_when_the_daemon_cannot_be_asked() {
    let codex_api = OneRead(Mutex::new(Some(Err(CallError::NotSent("the codex daemon is unreachable".into())))));
    let refusal = resolve_caller(&[codex("t1")], 1, &under_daemon(), &codex_api).await.unwrap_err();
    assert!(refusal.0.contains("could not verify"), "{}", refusal.0);
}

#[tokio::test]
async fn claude_caller_needs_no_daemon_call() {
    let env = FakeEnvironment { chain: vec![ancestor(1, 10), ancestor(50, 500)], sessions: vec![session(50, "s1", 500)], daemon: None };
    let identity = resolve_caller(&[claude(50, "s1")], 1, &env, &NoCalls).await.unwrap();
    assert_eq!(identity, Identity::Claude { session_id: "s1".into() });
}
