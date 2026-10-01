use std::sync::Arc;
use std::time::Duration;

use serde_json::json;

use super::super::codex_create::create_and_deliver;
use super::super::codex_resolve::NameMap;
use super::*;
use crate::outcome::Outcome;

const FRAME: &str = "<cross-session-message>hi</cross-session-message>";
const LONG_WAIT: Duration = Duration::from_secs(10);

#[tokio::test]
async fn naming_waits_for_turn_started_and_the_registration_precedes_the_turn() {
    let codex = Arc::new(FakeCodex::default());
    codex.hold_turn_started();
    codex
        .on("thread/start", Ok(json!({"thread": {"id": "new-1"}})))
        .on("turn/start", Ok(json!({"turn": {"id": "u1"}})))
        .on("thread/name/set", Ok(json!({})))
        .on("thread/unsubscribe", Ok(json!({})));
    let names = Arc::new(NameMap::default());

    let task = {
        let (codex, names) = (codex.clone(), names.clone());
        tokio::spawn(async move { create_and_deliver(&*codex, &names, LONG_WAIT, "worker", "F:\repo", MESSAGE_ID, FRAME).await })
    };
    while !codex.methods().contains(&"turn/start".to_string()) {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!codex.methods().contains(&"thread/name/set".to_string()), "named before turn/started arrived");

    codex.announce_turn_started("new-1");

    assert_eq!(task.await.unwrap(), Outcome::delivered());
    assert_eq!(codex.methods(), ["thread/start", "expect_turn_started", "turn/start", "thread/name/set", "thread/unsubscribe"]);
}
