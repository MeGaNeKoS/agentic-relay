use std::sync::Arc;

use super::*;
use crate::outcome::Outcome;
use crate::protocol::Target;

async fn send_to_cwd(cwd: &str) -> (Outcome, Arc<FakeCodex>) {
    let codex = Arc::new(FakeCodex::default());
    let bridge = bridge_for_claude_caller(&codex);
    let target = Target::CodexName { name: "worker".into(), cwd: cwd.into() };
    (bridge.send(1, MESSAGE_ID, &claude_caller(), &target, "hi").await, codex)
}

#[tokio::test]
async fn a_relative_cwd_is_refused_before_any_codex_call() {
    for cwd in [".", "repo", "..\\repo", "./repo", ""] {
        let (outcome, codex) = send_to_cwd(cwd).await;
        match outcome {
            Outcome::Failed(reason) => assert!(reason.contains("absolute"), "{cwd}: {reason}"),
            other => panic!("{cwd}: expected a failure, got {other:?}"),
        }
        assert!(codex.calls().is_empty(), "{cwd}");
    }
}
