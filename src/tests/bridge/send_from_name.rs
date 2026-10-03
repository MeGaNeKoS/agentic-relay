use std::sync::Arc;

use anyhow::Result;
use serde_json::json;

use super::*;
use crate::outcome::Outcome;
use crate::protocol::Target;

struct NamedClaudeCaller;

impl CallerEnvironment for NamedClaudeCaller {
    fn ancestry(&self, pid: u32) -> Result<Vec<Ancestor>> {
        ClaudeCallerEnvironment.ancestry(pid)
    }

    fn claude_session(&self, pid: u32) -> Result<Option<SessionFile>> {
        Ok(ClaudeCallerEnvironment.claude_session(pid)?.map(|file| SessionFile { name: Some("planner".into()), ..file }))
    }

    fn codex_daemon(&self) -> Result<Ancestor> {
        ClaudeCallerEnvironment.codex_daemon()
    }
}

#[tokio::test]
async fn a_named_sender_puts_from_name_in_the_delivered_frame() {
    let codex = Arc::new(FakeCodex::default());
    codex.on("thread/resume", Ok(idle())).on("turn/start", Ok(json!({"turn": {"id": "u1"}}))).on("thread/unsubscribe", Ok(json!({})));
    let bridge = bridge(&codex, Arc::new(NamedClaudeCaller));

    let outcome = bridge.send(1, MESSAGE_ID, &claude_caller(), &Target::CodexId { thread_id: "t1".into() }, "hi").await;

    assert_eq!(outcome, Outcome::delivered());
    let text = codex.params_of("turn/start")[0]["input"][0]["text"].as_str().unwrap().to_string();
    assert!(text.starts_with(&format!("<cross-session-message id=\"{MESSAGE_ID}\" from=\"claude {CALLER_SESSION}\" from-name=\"planner\">")), "{text}");
}
