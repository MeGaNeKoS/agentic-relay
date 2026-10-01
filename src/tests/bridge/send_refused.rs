use std::sync::Arc;

use serde_json::json;

use super::*;
use crate::outcome::Outcome;
use crate::protocol::Target;

fn reason(outcome: Outcome) -> String {
    match outcome {
        Outcome::Failed(reason) => reason,
        other => panic!("expected a failure, got {other:?}"),
    }
}

#[tokio::test]
async fn a_message_id_that_is_not_a_uuid_is_refused() {
    let codex = Arc::new(FakeCodex::default());
    let bridge = bridge_for_claude_caller(&codex);

    let outcome = bridge.send(1, "not-a-uuid\n", &claude_caller(), &Target::CodexId { thread_id: "t1".into() }, "hi").await;

    assert!(reason(outcome).contains("UUID"));
    assert!(codex.calls().is_empty());
}

#[tokio::test]
async fn an_empty_message_is_refused() {
    let codex = Arc::new(FakeCodex::default());
    let bridge = bridge_for_claude_caller(&codex);

    let outcome = bridge.send(1, MESSAGE_ID, &claude_caller(), &Target::CodexId { thread_id: "t1".into() }, "").await;

    assert!(reason(outcome).contains("empty"));
}

#[tokio::test]
async fn a_caller_with_no_trusted_identity_is_refused_before_anything_is_sent() {
    let codex = Arc::new(FakeCodex::default());
    let bridge = bridge_for_claude_caller(&codex);
    let stranger = vec![Candidate::Claude { pid: 999, session_id: "x".into() }];

    let outcome = bridge.send(1, MESSAGE_ID, &stranger, &Target::CodexId { thread_id: "t1".into() }, "hi").await;

    assert!(reason(outcome).starts_with("caller refused"));
    assert!(codex.calls().is_empty());
}

#[tokio::test]
async fn sending_to_your_own_claude_session_is_refused() {
    let codex = Arc::new(FakeCodex::default());
    let bridge = bridge_for_claude_caller(&codex);

    let outcome = bridge.send(1, MESSAGE_ID, &claude_caller(), &Target::Claude { session_id: CALLER_SESSION.into() }, "hi").await;

    assert!(reason(outcome).contains("yourself"));
}

#[tokio::test]
async fn sending_to_your_own_codex_thread_by_id_is_refused() {
    let codex = Arc::new(FakeCodex::default());
    codex.on("thread/read", Ok(json!({"thread": {"id": "me"}})));
    let bridge = bridge(&codex, Arc::new(CodexCallerEnvironment));
    let caller = vec![Candidate::Codex { thread_id: "me".into() }];

    let outcome = bridge.send(1, MESSAGE_ID, &caller, &Target::CodexId { thread_id: "me".into() }, "hi").await;

    assert!(reason(outcome).contains("yourself"));
    assert_eq!(codex.methods(), ["thread/read"]);
}

#[tokio::test]
async fn sending_to_your_own_codex_thread_by_name_is_refused() {
    let codex = Arc::new(FakeCodex::default());
    codex.on("thread/read", Ok(json!({"thread": {"id": "me"}}))).on("thread/list", Ok(listed(&[("me", Some("worker"), "f:/repo")])));
    let bridge = bridge(&codex, Arc::new(CodexCallerEnvironment));
    let caller = vec![Candidate::Codex { thread_id: "me".into() }];
    let target = Target::CodexName { name: "worker".into(), cwd: "F:\\repo".into() };

    let outcome = bridge.send(1, MESSAGE_ID, &caller, &target, "hi").await;

    assert!(reason(outcome).contains("yourself"));
    assert!(!codex.methods().contains(&"thread/resume".to_string()));
}

#[tokio::test]
async fn an_ambiguous_name_fails_without_creating_or_delivering() {
    let codex = Arc::new(FakeCodex::default());
    codex.on("thread/list", Ok(listed(&[("t1", Some("worker"), "f:/repo"), ("t2", Some("worker"), "f:/repo")])));
    let bridge = bridge_for_claude_caller(&codex);
    let target = Target::CodexName { name: "worker".into(), cwd: "f:/repo".into() };

    let outcome = bridge.send(1, MESSAGE_ID, &claude_caller(), &target, "hi").await;

    assert!(reason(outcome).starts_with("ambiguous"));
    assert_eq!(codex.methods(), ["thread/list"]);
}

#[tokio::test]
async fn a_claude_session_with_no_live_file_is_not_reachable() {
    let codex = Arc::new(FakeCodex::default());
    let bridge = bridge_for_claude_caller(&codex);

    let outcome = bridge.send(1, MESSAGE_ID, &claude_caller(), &Target::Claude { session_id: "nobody".into() }, "hi").await;

    assert!(reason(outcome).starts_with("not reachable"));
}
