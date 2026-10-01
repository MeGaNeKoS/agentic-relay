use std::sync::Arc;
use std::time::Duration;

use interprocess::local_socket::tokio::Stream;
use interprocess::local_socket::traits::tokio::{Listener as _, Stream as _};
use serde_json::json;

use super::super::serve::handle_client;
use super::*;
use crate::local_endpoint::{bind_name, endpoint_name_for, read_json_line, write_json_line};
use crate::protocol::{Request, Response, Target};

async fn exchange(bridge: Bridge, request_line: &str) -> Option<Response> {
    let printname = format!("relay-test-{}.sock", uuid::Uuid::new_v4());
    let listener = bind_name(endpoint_name_for(&printname).unwrap()).unwrap();
    let client = Stream::connect(endpoint_name_for(&printname).unwrap()).await.unwrap();
    let server_side = listener.accept().await.unwrap();
    let bridge = Arc::new(bridge);
    let handled = tokio::spawn(async move { handle_client(server_side, bridge).await });

    write_json_line(&client, request_line).await.unwrap();
    let line = read_json_line(&client, Duration::from_secs(10)).await.unwrap();
    handled.await.unwrap().unwrap();
    line.map(|line| serde_json::from_str(&line).unwrap())
}

fn send_request(text: &str) -> String {
    serde_json::to_string(&Request::Send {
        message_id: MESSAGE_ID.to_string(),
        caller_candidates: claude_caller(),
        target: Target::CodexId { thread_id: "t1".into() },
        text: text.to_string(),
    })
    .unwrap()
}

#[tokio::test]
async fn a_send_request_over_the_endpoint_is_answered_with_its_message_id() {
    let codex = Arc::new(FakeCodex::default());
    codex.on("thread/resume", Ok(idle())).on("turn/start", Ok(json!({"turn": {"id": "u1"}}))).on("thread/unsubscribe", Ok(json!({})));

    let response = exchange(bridge_for_claude_caller(&codex), &send_request("hi")).await;

    assert_eq!(response, Some(Response::Delivered { message_id: MESSAGE_ID.into(), note: None }));
}

#[tokio::test]
async fn a_failed_send_is_answered_with_its_reason() {
    let codex = Arc::new(FakeCodex::default());
    codex.on("thread/resume", Err(rejected("no rollout found for thread id t1"))).on("thread/unsubscribe", Ok(json!({})));

    let response = exchange(bridge_for_claude_caller(&codex), &send_request("hi")).await;

    let Some(Response::Failed { message_id, reason }) = response else { panic!("expected a failure, got {response:?}") };
    assert_eq!(message_id, MESSAGE_ID);
    assert!(reason.contains("no rollout found"), "{reason}");
}

#[tokio::test]
async fn a_line_that_is_not_a_request_is_answered_with_a_failure() {
    let codex = Arc::new(FakeCodex::default());

    let response = exchange(bridge_for_claude_caller(&codex), r#"{"action":"interrupt"}"#).await;

    let Some(Response::Failed { reason, .. }) = response else { panic!("expected a failure, got {response:?}") };
    assert!(reason.contains("malformed request"), "{reason}");
    assert!(codex.calls().is_empty());
}
