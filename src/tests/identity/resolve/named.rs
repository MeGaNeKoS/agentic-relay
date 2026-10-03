use serde_json::json;

use super::*;
use crate::identity::{Caller, Identity, resolve_caller};

#[tokio::test]
async fn a_named_claude_session_carries_its_name() {
    let named = SessionFile { name: Some("planner".into()), ..session(50, "s1", 500) };
    let env = FakeEnvironment { chain: vec![ancestor(1, 10), ancestor(50, 500)], sessions: vec![named], daemon: None };
    let caller = resolve_caller(&[claude(50, "s1")], 1, &env, &NoCalls).await.unwrap();
    assert_eq!(caller, Caller { identity: Identity::Claude { session_id: "s1".into() }, name: Some("planner".into()) });
}

#[tokio::test]
async fn a_named_codex_thread_carries_its_name() {
    let env = FakeEnvironment { chain: vec![ancestor(1, 10), ancestor(60, 600)], sessions: vec![], daemon: Some(ancestor(60, 600)) };
    let caller = resolve_caller(&[codex("t1")], 1, &env, &read_of(json!({"thread": {"name": "worker"}}))).await.unwrap();
    assert_eq!(caller, Caller { identity: Identity::Codex { thread_id: "t1".into() }, name: Some("worker".into()) });
}

#[tokio::test]
async fn a_codex_subagent_is_named_by_its_nickname() {
    let env = FakeEnvironment { chain: vec![ancestor(1, 10), ancestor(60, 600)], sessions: vec![], daemon: Some(ancestor(60, 600)) };
    let caller = resolve_caller(&[codex("t1")], 1, &env, &read_of(json!({"thread": {"name": null, "agentNickname": "Kepler"}}))).await.unwrap();
    assert_eq!(caller.name.as_deref(), Some("Kepler"));
}

#[tokio::test]
async fn a_codex_thread_name_wins_over_its_nickname() {
    let env = FakeEnvironment { chain: vec![ancestor(1, 10), ancestor(60, 600)], sessions: vec![], daemon: Some(ancestor(60, 600)) };
    let caller = resolve_caller(&[codex("t1")], 1, &env, &read_of(json!({"thread": {"name": "worker", "agentNickname": "Kepler"}}))).await.unwrap();
    assert_eq!(caller.name.as_deref(), Some("worker"));
}

#[tokio::test]
async fn the_name_is_cleaned_before_it_is_carried() {
    let named = SessionFile { name: Some("  \"a<b>\"\u{200B}  ".into()), ..session(50, "s1", 500) };
    let env = FakeEnvironment { chain: vec![ancestor(1, 10), ancestor(50, 500)], sessions: vec![named], daemon: None };
    let caller = resolve_caller(&[claude(50, "s1")], 1, &env, &NoCalls).await.unwrap();
    assert_eq!(caller.name.as_deref(), Some("ab"));
}
