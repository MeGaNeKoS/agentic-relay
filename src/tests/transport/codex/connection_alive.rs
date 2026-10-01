use serde_json::json;

use super::*;
use crate::transport::codex::CallError;

#[tokio::test]
async fn a_call_gets_its_answer() {
    let daemon = FakeDaemon::start(AfterInitialize::Answer);
    let client = daemon.connect().await;

    let result = client.call("thread/list", json!({})).await.unwrap();

    assert_eq!(result["echo"], "thread/list");
    assert!(!client.is_closed());
}

#[tokio::test]
async fn the_handshake_sends_initialize_then_initialized() {
    let daemon = FakeDaemon::start(AfterInitialize::Answer);
    let client = daemon.connect().await;
    client.call("thread/list", json!({})).await.unwrap();

    let methods: Vec<String> = daemon.received.lock().unwrap().iter().filter_map(|f| f["method"].as_str().map(str::to_string)).collect();

    assert_eq!(methods, ["initialize", "initialized", "thread/list"]);
}

#[tokio::test]
async fn a_daemon_error_is_a_rejection_carrying_its_message() {
    let daemon = FakeDaemon::start(AfterInitialize::Answer);
    let client = daemon.connect().await;

    let Err(CallError::Rejected(error)) = client.call("fail", json!({})).await else { panic!("expected a rejection") };

    assert_eq!(error.message, "no good");
    assert!(!client.is_closed());
}

#[tokio::test]
async fn an_ask_from_the_daemon_is_never_answered() {
    let daemon = FakeDaemon::start(AfterInitialize::AskThenAnswer);
    let client = daemon.connect().await;

    assert_eq!(client.call("thread/list", json!({})).await.unwrap()["echo"], "thread/list");
    assert_eq!(client.call("thread/read", json!({})).await.unwrap()["echo"], "thread/read");

    let answered_the_ask = daemon.received.lock().unwrap().iter().any(|f| f["id"] == 99);
    assert!(!answered_the_ask, "relay must not reply to an ask meant for another connection");
}

#[tokio::test]
async fn a_turn_started_notification_resolves_only_the_waiter_for_its_thread() {
    let daemon = FakeDaemon::start(AfterInitialize::NotifyTurnStarted);
    let client = daemon.connect().await;
    let waiter = client.expect_turn_started("t-9");
    let other = client.expect_turn_started("t-7");

    client.call("turn/start", json!({})).await.unwrap();

    assert!(waiter.wait(std::time::Duration::from_secs(5)).await);
    assert!(!other.wait(std::time::Duration::from_millis(100)).await);
}
