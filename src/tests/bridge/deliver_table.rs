use serde_json::json;

use super::super::codex_deliver::deliver;
use super::*;
use crate::outcome::Outcome;

const FRAME: &str = "<cross-session-message>hi</cross-session-message>";

fn assert_unsubscribed_last(codex: &FakeCodex) {
    assert_eq!(codex.methods().last().map(String::as_str), Some("thread/unsubscribe"));
}

#[tokio::test]
async fn idle_thread_gets_a_turn_with_no_overrides() {
    let codex = FakeCodex::default();
    codex.on("thread/resume", Ok(idle())).on("turn/start", Ok(json!({"turn": {"id": "u1"}}))).on("thread/unsubscribe", Ok(json!({})));

    assert_eq!(deliver(&codex, "t1", MESSAGE_ID, FRAME).await, Outcome::delivered());

    assert_eq!(codex.methods(), ["thread/resume", "turn/start", "thread/unsubscribe"]);
    let start = &codex.params_of("turn/start")[0];
    assert_eq!(start["threadId"], "t1");
    assert_eq!(start["clientUserMessageId"], MESSAGE_ID);
    assert_eq!(start["input"][0]["text"], FRAME);
    for override_key in ["model", "cwd", "approvalPolicy", "sandboxPolicy", "sandbox"] {
        assert!(start.get(override_key).is_none(), "{override_key} must not be passed on turn/start");
    }
}

#[tokio::test]
async fn active_thread_is_steered_into_the_running_turn() {
    let codex = FakeCodex::default();
    codex.on("thread/resume", Ok(active("turn-9"))).on("turn/steer", Ok(json!({"turnId": "turn-9"}))).on("thread/unsubscribe", Ok(json!({})));

    assert_eq!(deliver(&codex, "t1", MESSAGE_ID, FRAME).await, Outcome::delivered());

    assert_eq!(codex.methods(), ["thread/resume", "turn/steer", "thread/unsubscribe"]);
    let steer = &codex.params_of("turn/steer")[0];
    assert_eq!(steer["expectedTurnId"], "turn-9");
    assert_eq!(steer["clientUserMessageId"], MESSAGE_ID);
}

#[tokio::test]
async fn a_turn_that_cannot_be_steered_is_queued() {
    let codex = FakeCodex::default();
    codex
        .on("thread/resume", Ok(active("turn-9")))
        .on("turn/steer", Err(not_steerable()))
        .on("thread/queue/add", Ok(json!({"queuedSubmission": {"id": "q1"}})))
        .on("thread/unsubscribe", Ok(json!({})));

    assert_eq!(deliver(&codex, "t1", MESSAGE_ID, FRAME).await, Outcome::delivered());

    assert_eq!(codex.methods(), ["thread/resume", "turn/steer", "thread/queue/add", "thread/unsubscribe"]);
    assert_eq!(codex.params_of("thread/queue/add")[0]["clientUserMessageId"], MESSAGE_ID);
}

#[tokio::test]
async fn a_finished_turn_is_retried_once_from_a_fresh_resume() {
    let codex = FakeCodex::default();
    codex
        .on("thread/resume", Ok(active("turn-9")))
        .on("turn/steer", Err(rejected("no active turn to steer")))
        .on("thread/resume", Ok(idle()))
        .on("turn/start", Ok(json!({"turn": {"id": "u1"}})))
        .on("thread/unsubscribe", Ok(json!({})));

    assert_eq!(deliver(&codex, "t1", MESSAGE_ID, FRAME).await, Outcome::delivered());

    assert_eq!(codex.methods(), ["thread/resume", "turn/steer", "thread/resume", "turn/start", "thread/unsubscribe"]);
    assert!(codex.params_of("thread/queue/add").is_empty());
    assert_unsubscribed_last(&codex);
}

#[tokio::test]
async fn a_steer_error_on_a_turn_that_is_still_running_queues_the_message() {
    let codex = FakeCodex::default();
    codex
        .on("thread/resume", Ok(active("turn-9")))
        .on("turn/steer", Err(rejected("no active turn to steer")))
        .on("thread/resume", Ok(active("turn-9")))
        .on("thread/queue/add", Ok(json!({"queuedSubmission": {"id": "q1"}})))
        .on("thread/unsubscribe", Ok(json!({})));

    assert_eq!(deliver(&codex, "t1", MESSAGE_ID, FRAME).await, Outcome::delivered());

    assert_eq!(codex.methods(), ["thread/resume", "turn/steer", "thread/resume", "thread/queue/add", "thread/unsubscribe"]);
    assert_eq!(codex.params_of("thread/queue/add")[0]["clientUserMessageId"], MESSAGE_ID);
}

#[tokio::test]
async fn the_resume_asks_for_the_newest_turn_only() {
    let codex = FakeCodex::default();
    codex.on("thread/resume", Ok(idle())).on("turn/start", Ok(json!({"turn": {"id": "u1"}}))).on("thread/unsubscribe", Ok(json!({})));

    deliver(&codex, "t1", MESSAGE_ID, FRAME).await;

    let resume = &codex.params_of("thread/resume")[0];
    assert_eq!(resume["excludeTurns"], true);
    assert_eq!(resume["initialTurnsPage"]["limit"], 1);
}

#[tokio::test]
async fn a_replaced_turn_is_retried_once_from_a_fresh_resume() {
    let codex = FakeCodex::default();
    codex
        .on("thread/resume", Ok(active("turn-9")))
        .on("turn/steer", Err(rejected("expected active turn id turn-9 but found turn-10")))
        .on("thread/resume", Ok(active("turn-10")))
        .on("turn/steer", Ok(json!({"turnId": "turn-10"})))
        .on("thread/unsubscribe", Ok(json!({})));

    assert_eq!(deliver(&codex, "t1", MESSAGE_ID, FRAME).await, Outcome::delivered());

    let steers = codex.params_of("turn/steer");
    assert_eq!(steers.len(), 2);
    assert_eq!(steers[1]["expectedTurnId"], "turn-10");
}
