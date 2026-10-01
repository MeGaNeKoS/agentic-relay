use serde_json::json;

use super::super::codex_deliver::deliver;
use super::*;
use crate::outcome::Outcome;

const FRAME: &str = "<cross-session-message>hi</cross-session-message>";

fn failure_reason(outcome: Outcome) -> String {
    match outcome {
        Outcome::Failed(reason) => reason,
        other => panic!("expected a failure, got {other:?}"),
    }
}

#[tokio::test]
async fn system_error_status_fails_without_a_handoff() {
    let codex = FakeCodex::default();
    codex.on("thread/resume", Ok(resumed(json!({"type": "systemError"}), json!([])))).on("thread/unsubscribe", Ok(json!({})));

    let reason = failure_reason(deliver(&codex, "t1", MESSAGE_ID, FRAME).await);

    assert!(reason.contains("system error"), "{reason}");
    assert_eq!(codex.methods(), ["thread/resume", "thread/unsubscribe"]);
}

#[tokio::test]
async fn unexpected_status_fails_without_a_handoff() {
    let codex = FakeCodex::default();
    codex.on("thread/resume", Ok(resumed(json!({"type": "notLoaded"}), json!([])))).on("thread/unsubscribe", Ok(json!({})));

    let reason = failure_reason(deliver(&codex, "t1", MESSAGE_ID, FRAME).await);

    assert!(reason.contains("notLoaded"), "{reason}");
    assert!(!codex.methods().contains(&"turn/start".to_string()));
}

#[tokio::test]
async fn thread_with_no_rollout_fails_with_the_daemon_message() {
    let codex = FakeCodex::default();
    codex.on("thread/resume", Err(rejected("no rollout found for thread id t1"))).on("thread/unsubscribe", Ok(json!({})));

    let reason = failure_reason(deliver(&codex, "t1", MESSAGE_ID, FRAME).await);

    assert!(reason.contains("no rollout found"), "{reason}");
}

#[tokio::test]
async fn unreachable_daemon_fails() {
    let codex = FakeCodex::default();
    codex.on("thread/resume", Err(CallError::NotSent("the codex daemon is unreachable".into()))).on("thread/unsubscribe", Err(CallError::NotSent("unreachable".into())));

    let reason = failure_reason(deliver(&codex, "t1", MESSAGE_ID, FRAME).await);

    assert!(reason.contains("unreachable"), "{reason}");
}

#[tokio::test]
async fn lost_resume_answer_fails_because_nothing_was_handed_off() {
    let codex = FakeCodex::default();
    codex.on("thread/resume", Err(lost("no answer"))).on("thread/unsubscribe", Ok(json!({})));

    assert!(matches!(deliver(&codex, "t1", MESSAGE_ID, FRAME).await, Outcome::Failed(_)));
}

#[tokio::test]
async fn a_turn_that_keeps_moving_fails_as_target_busy() {
    let codex = FakeCodex::default();
    codex
        .on("thread/resume", Ok(active("turn-9")))
        .on("turn/steer", Err(rejected("no active turn to steer")))
        .on("thread/resume", Ok(active("turn-10")))
        .on("turn/steer", Err(rejected("expected active turn id `turn-10` but found `turn-11`")))
        .on("thread/unsubscribe", Ok(json!({})));

    assert_eq!(failure_reason(deliver(&codex, "t1", MESSAGE_ID, FRAME).await), "target busy, retry");
    assert_eq!(codex.params_of("thread/resume").len(), 2);
    assert!(codex.params_of("thread/queue/add").is_empty());
}

#[tokio::test]
async fn active_status_with_no_running_turn_is_retried_then_busy() {
    let codex = FakeCodex::default();
    codex
        .on("thread/resume", Ok(resumed(json!({"type": "active", "activeFlags": []}), json!([]))))
        .on("thread/resume", Ok(resumed(json!({"type": "active", "activeFlags": []}), json!([]))))
        .on("thread/unsubscribe", Ok(json!({})));

    assert_eq!(failure_reason(deliver(&codex, "t1", MESSAGE_ID, FRAME).await), "target busy, retry");
}

#[tokio::test]
async fn lost_turn_start_answer_may_not_have_landed_and_is_never_resent() {
    let codex = FakeCodex::default();
    codex.on("thread/resume", Ok(idle())).on("turn/start", Err(lost("turn/start got no answer"))).on("thread/unsubscribe", Ok(json!({})));

    assert!(matches!(deliver(&codex, "t1", MESSAGE_ID, FRAME).await, Outcome::MayNotHaveLanded(_)));
    assert_eq!(codex.params_of("turn/start").len(), 1);
}

#[tokio::test]
async fn lost_steer_answer_may_not_have_landed_and_is_not_queued() {
    let codex = FakeCodex::default();
    codex.on("thread/resume", Ok(active("turn-9"))).on("turn/steer", Err(lost("dropped"))).on("thread/unsubscribe", Ok(json!({})));

    assert!(matches!(deliver(&codex, "t1", MESSAGE_ID, FRAME).await, Outcome::MayNotHaveLanded(_)));
    assert!(codex.params_of("thread/queue/add").is_empty());
}

#[tokio::test]
async fn lost_queue_answer_may_not_have_landed() {
    let codex = FakeCodex::default();
    codex
        .on("thread/resume", Ok(active("turn-9")))
        .on("turn/steer", Err(not_steerable()))
        .on("thread/queue/add", Err(lost("dropped")))
        .on("thread/unsubscribe", Ok(json!({})));

    assert!(matches!(deliver(&codex, "t1", MESSAGE_ID, FRAME).await, Outcome::MayNotHaveLanded(_)));
}

#[tokio::test]
async fn any_other_steer_error_fails_with_the_daemon_message_and_is_not_queued() {
    let codex = FakeCodex::default();
    codex
        .on("thread/resume", Ok(active("turn-9")))
        .on("turn/steer", Err(rejected("direct app-server input is not allowed for multi-agent v2 sub-agents")))
        .on("thread/unsubscribe", Ok(json!({})));

    let reason = failure_reason(deliver(&codex, "t1", MESSAGE_ID, FRAME).await);

    assert!(reason.contains("multi-agent v2 sub-agents"), "{reason}");
    assert!(codex.params_of("thread/queue/add").is_empty());
}

#[tokio::test]
async fn relay_unsubscribes_after_a_failure_too() {
    let codex = FakeCodex::default();
    codex.on("thread/resume", Err(rejected("thread is archived"))).on("thread/unsubscribe", Ok(json!({})));

    deliver(&codex, "t1", MESSAGE_ID, FRAME).await;

    assert_eq!(codex.methods().last().map(String::as_str), Some("thread/unsubscribe"));
}
