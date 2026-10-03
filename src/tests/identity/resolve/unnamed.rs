use serde_json::json;

use super::*;
use crate::identity::resolve_caller;

#[tokio::test]
async fn a_claude_session_without_a_name_has_none() {
    let env = FakeEnvironment { chain: vec![ancestor(1, 10), ancestor(50, 500)], sessions: vec![session(50, "s1", 500)], daemon: None };
    let caller = resolve_caller(&[claude(50, "s1")], 1, &env, &NoCalls).await.unwrap();
    assert_eq!(caller.name, None);
}

#[tokio::test]
async fn a_claude_name_that_cleans_to_empty_is_none() {
    let named = SessionFile { name: Some("\"<>\"\u{2028}".into()), ..session(50, "s1", 500) };
    let env = FakeEnvironment { chain: vec![ancestor(1, 10), ancestor(50, 500)], sessions: vec![named], daemon: None };
    let caller = resolve_caller(&[claude(50, "s1")], 1, &env, &NoCalls).await.unwrap();
    assert_eq!(caller.name, None);
}

#[tokio::test]
async fn a_codex_thread_without_a_name_has_none() {
    let env = FakeEnvironment { chain: vec![ancestor(1, 10), ancestor(60, 600)], sessions: vec![], daemon: Some(ancestor(60, 600)) };
    let caller = resolve_caller(&[codex("t1")], 1, &env, &read_of(json!({"thread": {"id": "t1"}}))).await.unwrap();
    assert_eq!(caller.name, None);
}

#[tokio::test]
async fn a_codex_name_that_is_null_or_blank_is_none() {
    let env = FakeEnvironment { chain: vec![ancestor(1, 10), ancestor(60, 600)], sessions: vec![], daemon: Some(ancestor(60, 600)) };
    for name in [json!(null), json!("   ")] {
        let caller = resolve_caller(&[codex("t1")], 1, &env, &read_of(json!({"thread": {"name": name}}))).await.unwrap();
        assert_eq!(caller.name, None);
    }
}
