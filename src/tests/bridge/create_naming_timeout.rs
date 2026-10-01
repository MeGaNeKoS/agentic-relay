use serde_json::json;

use super::super::codex_create::create_and_deliver;
use super::super::codex_resolve::{Lookup, NameMap, lookup};
use super::*;
use crate::outcome::Outcome;

const FRAME: &str = "<cross-session-message>hi</cross-session-message>";

#[tokio::test]
async fn no_turn_started_in_time_is_delivered_with_the_unnamed_note_and_no_naming_call() {
    let codex = FakeCodex::default();
    codex.hold_turn_started();
    codex
        .on("thread/start", Ok(json!({"thread": {"id": "new-1"}})))
        .on("turn/start", Ok(json!({"turn": {"id": "u1"}})))
        .on("thread/unsubscribe", Ok(json!({})));
    let names = NameMap::default();

    let outcome = create_and_deliver(&codex, &names, NAMING_WAIT, "worker", "F:\repo", MESSAGE_ID, FRAME).await;

    let Outcome::Delivered { note: Some(note) } = outcome else { panic!("expected delivered with a note, got {outcome:?}") };
    assert!(note.contains("new-1") && note.contains("unnamed"), "{note}");
    assert!(!codex.methods().contains(&"thread/name/set".to_string()));
    codex.on("thread/list", Ok(listed(&[])));
    assert_eq!(lookup(&codex, &names, "worker", "F:\repo").await, Ok(Lookup::Missing));
}
