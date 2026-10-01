use std::sync::Arc;

use serde_json::json;

use super::*;
use crate::outcome::Outcome;
use crate::protocol::Target;

#[tokio::test]
async fn codex_id_send_delivers_a_frame_carrying_the_message_id_and_the_reply_address() {
    let codex = Arc::new(FakeCodex::default());
    codex.on("thread/resume", Ok(idle())).on("turn/start", Ok(json!({"turn": {"id": "u1"}}))).on("thread/unsubscribe", Ok(json!({})));
    let bridge = bridge_for_claude_caller(&codex);

    let outcome = bridge.send(1, MESSAGE_ID, &claude_caller(), &Target::CodexId { thread_id: "t1".into() }, "do the thing").await;

    assert_eq!(outcome, Outcome::delivered());
    let text = codex.params_of("turn/start")[0]["input"][0]["text"].as_str().unwrap().to_string();
    assert!(text.starts_with(&format!("<cross-session-message id=\"{MESSAGE_ID}\" from=\"claude {CALLER_SESSION}\">")), "{text}");
    assert!(text.contains("do the thing"));
}

#[tokio::test]
async fn codex_name_send_resolves_then_delivers_into_the_found_thread() {
    let codex = Arc::new(FakeCodex::default());
    codex
        .on("thread/list", Ok(listed(&[("t1", Some("worker"), "f:/repo")])))
        .on("thread/resume", Ok(idle()))
        .on("turn/start", Ok(json!({"turn": {"id": "u1"}})))
        .on("thread/unsubscribe", Ok(json!({})));
    let bridge = bridge_for_claude_caller(&codex);

    let target = Target::CodexName { name: "worker".into(), cwd: "F:\\repo".into() };
    assert_eq!(bridge.send(1, MESSAGE_ID, &claude_caller(), &target, "hi").await, Outcome::delivered());

    assert_eq!(codex.methods(), ["thread/list", "thread/resume", "turn/start", "thread/unsubscribe"]);
    assert_eq!(codex.params_of("thread/resume")[0]["threadId"], "t1");
}

#[tokio::test]
async fn codex_name_send_to_a_missing_thread_creates_it() {
    let codex = Arc::new(FakeCodex::default());
    codex
        .on("thread/list", Ok(listed(&[])))
        .on("thread/start", Ok(json!({"thread": {"id": "new-1"}})))
        .on("turn/start", Ok(json!({"turn": {"id": "u1"}})))
        .on("thread/name/set", Ok(json!({})))
        .on("thread/unsubscribe", Ok(json!({})));
    let bridge = bridge_for_claude_caller(&codex);

    let target = Target::CodexName { name: "worker".into(), cwd: "F:\\repo".into() };
    assert_eq!(bridge.send(1, MESSAGE_ID, &claude_caller(), &target, "hi").await, Outcome::delivered());

    assert_eq!(codex.methods(), ["thread/list", "thread/start", "expect_turn_started", "turn/start", "thread/name/set", "thread/unsubscribe"]);
}

#[tokio::test]
async fn a_codex_caller_can_send_to_another_thread() {
    let codex = Arc::new(FakeCodex::default());
    codex
        .on("thread/read", Ok(json!({"thread": {"id": "me"}})))
        .on("thread/resume", Ok(idle()))
        .on("turn/start", Ok(json!({"turn": {"id": "u1"}})))
        .on("thread/unsubscribe", Ok(json!({})));
    let bridge = bridge(&codex, Arc::new(CodexCallerEnvironment));

    let caller = vec![Candidate::Codex { thread_id: "me".into() }];
    let outcome = bridge.send(1, MESSAGE_ID, &caller, &Target::CodexId { thread_id: "other".into() }, "hi").await;

    assert_eq!(outcome, Outcome::delivered());
    let text = codex.params_of("turn/start")[0]["input"][0]["text"].as_str().unwrap().to_string();
    assert!(text.contains("from=\"codex me\""), "{text}");
}

#[tokio::test]
async fn sends_to_one_thread_are_handed_off_in_the_order_they_arrived() {
    let codex = Arc::new(FakeCodex::default());
    for _ in 0..3 {
        codex.on("thread/resume", Ok(idle())).on("turn/start", Ok(json!({"turn": {"id": "u"}}))).on("thread/unsubscribe", Ok(json!({})));
    }
    let bridge = Arc::new(bridge_for_claude_caller(&codex));

    let mut sends = Vec::new();
    for text in ["first", "second", "third"] {
        let bridge = bridge.clone();
        sends.push(tokio::spawn(async move {
            bridge.send(1, MESSAGE_ID, &claude_caller(), &Target::CodexId { thread_id: "t1".into() }, text).await
        }));
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    for send in sends {
        assert_eq!(send.await.unwrap(), Outcome::delivered());
    }

    let order: Vec<String> = codex.params_of("turn/start").iter().map(|p| p["input"][0]["text"].as_str().unwrap().to_string()).collect();
    assert!(order[0].contains("first") && order[1].contains("second") && order[2].contains("third"), "{order:?}");
    let methods = codex.methods();
    assert_eq!(methods[..3], ["thread/resume", "turn/start", "thread/unsubscribe"], "each send finishes before the next starts");
}
