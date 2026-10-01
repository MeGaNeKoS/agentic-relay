use serde_json::json;

use super::super::codex_create::create_and_deliver;
use super::super::codex_resolve::{Lookup, NameMap, lookup};
use super::*;
use crate::outcome::Outcome;

const FRAME: &str = "<cross-session-message>hi</cross-session-message>";
const CWD: &str = "F:\\repo\\app";

#[tokio::test]
async fn thread_start_answer_lost_may_not_have_landed_and_nothing_is_sent() {
    let codex = FakeCodex::default();
    codex.on("thread/start", Err(lost("no answer")));

    let outcome = create_and_deliver(&codex, &NameMap::default(), NAMING_WAIT, "worker", CWD, MESSAGE_ID, FRAME).await;

    assert!(matches!(outcome, Outcome::MayNotHaveLanded(_)), "{outcome:?}");
    assert_eq!(codex.methods(), ["thread/start"]);
}

#[tokio::test]
async fn thread_start_rejected_fails_with_the_daemon_message() {
    let codex = FakeCodex::default();
    codex.on("thread/start", Err(rejected("model unavailable")));

    let outcome = create_and_deliver(&codex, &NameMap::default(), NAMING_WAIT, "worker", CWD, MESSAGE_ID, FRAME).await;

    let Outcome::Failed(reason) = outcome else { panic!("expected a failure, got {outcome:?}") };
    assert!(reason.contains("model unavailable"), "{reason}");
}

#[tokio::test]
async fn turn_start_rejected_fails_leaves_the_thread_unnamed_and_unsubscribes() {
    let codex = FakeCodex::default();
    codex
        .on("thread/start", Ok(json!({"thread": {"id": "new-1"}})))
        .on("turn/start", Err(rejected("turn refused")))
        .on("thread/unsubscribe", Ok(json!({})));
    let names = NameMap::default();

    let outcome = create_and_deliver(&codex, &names, NAMING_WAIT, "worker", CWD, MESSAGE_ID, FRAME).await;

    let Outcome::Failed(reason) = outcome else { panic!("expected a failure, got {outcome:?}") };
    assert!(reason.contains("turn refused"), "{reason}");
    assert_eq!(codex.methods(), ["thread/start", "expect_turn_started", "turn/start", "thread/unsubscribe"]);
    codex.on("thread/list", Ok(listed(&[])));
    assert_eq!(lookup(&codex, &names, "worker", CWD).await, Ok(Lookup::Missing));
}

#[tokio::test]
async fn turn_start_answer_lost_may_not_have_landed_names_the_thread_and_leaves_it_unnamed() {
    let codex = FakeCodex::default();
    codex
        .on("thread/start", Ok(json!({"thread": {"id": "new-1"}})))
        .on("turn/start", Err(lost("no answer")))
        .on("thread/unsubscribe", Ok(json!({})));
    let names = NameMap::default();

    let outcome = create_and_deliver(&codex, &names, NAMING_WAIT, "worker", CWD, MESSAGE_ID, FRAME).await;

    let Outcome::MayNotHaveLanded(reason) = outcome else { panic!("expected may not have landed, got {outcome:?}") };
    assert!(reason.contains("new-1"), "{reason}");
    assert_eq!(codex.methods(), ["thread/start", "expect_turn_started", "turn/start", "thread/unsubscribe"]);
    assert_eq!(codex.params_of("turn/start").len(), 1);
    codex.on("thread/list", Ok(listed(&[])));
    assert_eq!(lookup(&codex, &names, "worker", CWD).await, Ok(Lookup::Missing));
}

#[tokio::test]
async fn a_start_answer_with_no_thread_id_fails() {
    let codex = FakeCodex::default();
    codex.on("thread/start", Ok(json!({"thread": {}})));

    let outcome = create_and_deliver(&codex, &NameMap::default(), NAMING_WAIT, "worker", CWD, MESSAGE_ID, FRAME).await;

    assert!(matches!(outcome, Outcome::Failed(_)), "{outcome:?}");
    assert_eq!(codex.methods(), ["thread/start"]);
}
