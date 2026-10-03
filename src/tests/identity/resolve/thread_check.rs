use serde_json::json;

use super::*;
use crate::identity::{Identity, resolve_caller};
use crate::transport::codex::jsonrpc::RpcError;
use crate::transport::codex::CallError;

fn under_daemon() -> FakeEnvironment {
    FakeEnvironment { chain: vec![ancestor(1, 10), ancestor(60, 600)], sessions: vec![], daemon: Some(ancestor(60, 600)) }
}

#[tokio::test]
async fn codex_caller_is_accepted_when_the_thread_exists() {
    let codex_api = OneRead(Mutex::new(Some(Ok(json!({"thread": {}})))));
    let identity = resolve_caller(&[codex("t1")], 1, &under_daemon(), &codex_api).await.unwrap().identity;
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
    let identity = resolve_caller(&[claude(50, "s1")], 1, &env, &NoCalls).await.unwrap().identity;
    assert_eq!(identity, Identity::Claude { session_id: "s1".into() });
}
