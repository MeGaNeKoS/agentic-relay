use serde_json::json;

use super::super::codex_create::create_and_deliver;
use super::super::codex_resolve::{Lookup, NameMap, lookup};
use super::*;
use crate::outcome::Outcome;

const FRAME: &str = "<cross-session-message>hi</cross-session-message>";
const CWD: &str = "F:\\repo\\app";

fn scripted_success(codex: &FakeCodex) {
    codex
        .on("thread/start", Ok(json!({"thread": {"id": "new-1"}})))
        .on("turn/start", Ok(json!({"turn": {"id": "u1"}})))
        .on("thread/name/set", Ok(json!({})))
        .on("thread/unsubscribe", Ok(json!({})));
}

#[tokio::test]
async fn a_missing_thread_is_started_delivered_into_then_named_in_that_order() {
    let codex = FakeCodex::default();
    scripted_success(&codex);
    let names = NameMap::default();

    let outcome = create_and_deliver(&codex, &names, NAMING_WAIT, "worker", CWD, MESSAGE_ID, FRAME).await;

    assert_eq!(outcome, Outcome::delivered());
    assert_eq!(codex.methods(), ["thread/start", "expect_turn_started", "turn/start", "thread/name/set", "thread/unsubscribe"]);
}

#[tokio::test]
async fn the_new_thread_is_started_with_only_its_cwd_and_the_turn_gets_none() {
    let codex = FakeCodex::default();
    scripted_success(&codex);

    create_and_deliver(&codex, &NameMap::default(), NAMING_WAIT, "worker", CWD, MESSAGE_ID, FRAME).await;

    assert_eq!(codex.params_of("thread/start")[0], json!({"cwd": CWD}));
    let turn = &codex.params_of("turn/start")[0];
    assert_eq!(turn["threadId"], "new-1");
    assert_eq!(turn["clientUserMessageId"], MESSAGE_ID);
    for override_key in ["model", "cwd", "approvalPolicy", "sandboxPolicy", "sandbox"] {
        assert!(turn.get(override_key).is_none(), "{override_key} must not be passed on turn/start");
    }
    let naming = &codex.params_of("thread/name/set")[0];
    assert_eq!((naming["threadId"].as_str(), naming["name"].as_str()), (Some("new-1"), Some("worker")));
}

#[tokio::test]
async fn once_named_the_thread_is_in_the_map_for_the_next_send() {
    let codex = FakeCodex::default();
    scripted_success(&codex);
    let names = NameMap::default();
    create_and_deliver(&codex, &names, NAMING_WAIT, "worker", CWD, MESSAGE_ID, FRAME).await;

    codex
        .on("thread/list", Ok(listed(&[])))
        .on("thread/read", Ok(json!({"thread": {"id": "new-1", "name": "worker", "cwd": "f:/repo/app"}})));

    assert_eq!(lookup(&codex, &names, "worker", CWD).await, Ok(Lookup::Found("new-1".into())));
}

#[tokio::test]
async fn naming_that_fails_after_delivery_is_delivered_with_a_note_and_no_map_entry() {
    let codex = FakeCodex::default();
    codex
        .on("thread/start", Ok(json!({"thread": {"id": "new-1"}})))
        .on("turn/start", Ok(json!({"turn": {"id": "u1"}})))
        .on("thread/name/set", Err(rejected("cannot name")))
        .on("thread/unsubscribe", Ok(json!({})));
    let names = NameMap::default();

    let outcome = create_and_deliver(&codex, &names, NAMING_WAIT, "worker", CWD, MESSAGE_ID, FRAME).await;

    let Outcome::Delivered { note: Some(note) } = outcome else { panic!("expected delivered with a note, got {outcome:?}") };
    assert!(note.contains("new-1") && note.contains("unnamed"), "{note}");
    codex.on("thread/list", Ok(listed(&[])));
    assert_eq!(lookup(&codex, &names, "worker", CWD).await, Ok(Lookup::Missing));
    assert_eq!(codex.params_of("thread/read").len(), 0);
}
