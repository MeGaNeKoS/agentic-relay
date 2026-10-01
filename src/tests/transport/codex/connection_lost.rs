use std::time::Duration;

use serde_json::json;

use super::*;
use crate::transport::codex::CallError;

const WELL_BEFORE_ANY_HANG: Duration = Duration::from_secs(10);

async fn within_bound<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(WELL_BEFORE_ANY_HANG, future).await.expect("the call hung")
}

#[tokio::test]
async fn a_call_pending_when_the_daemon_drops_fails_as_lost_instead_of_hanging() {
    let daemon = FakeDaemon::start(AfterInitialize::DropOnFirstRequest);
    let client = daemon.connect().await;

    let result = within_bound(client.call("thread/list", json!({}))).await;

    assert!(matches!(result, Err(CallError::Lost(_))), "{result:?}");
}

#[tokio::test]
async fn after_the_connection_dies_later_calls_fail_fast_as_not_sent() {
    let daemon = FakeDaemon::start(AfterInitialize::DropOnFirstRequest);
    let client = daemon.connect().await;
    let _ = within_bound(client.call("thread/list", json!({}))).await;

    let result = within_bound(client.call("thread/read", json!({}))).await;

    assert!(matches!(result, Err(CallError::NotSent(_))), "{result:?}");
    assert!(client.is_closed());
}

#[tokio::test]
async fn a_malformed_frame_ends_the_connection_and_fails_the_pending_call() {
    let daemon = FakeDaemon::start(AfterInitialize::GarbageOnFirstRequest);
    let client = daemon.connect().await;

    let result = within_bound(client.call("thread/list", json!({}))).await;

    assert!(matches!(result, Err(CallError::Lost(_))), "{result:?}");
    assert!(client.is_closed());
}

#[tokio::test]
async fn a_daemon_that_never_answers_times_the_call_out_as_lost_and_the_connection_stays_usable() {
    let daemon = FakeDaemon::start(AfterInitialize::Silent);
    let client = daemon.connect().await;

    let result = within_bound(client.call("thread/list", json!({}))).await;

    let Err(CallError::Lost(reason)) = result else { panic!("expected lost, got {result:?}") };
    assert!(reason.contains("no answer"), "{reason}");
    assert!(!client.is_closed());
}
