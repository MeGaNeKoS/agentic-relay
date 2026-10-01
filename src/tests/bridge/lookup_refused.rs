use serde_json::json;

use super::super::codex_resolve::{NameMap, lookup};
use super::*;
use crate::outcome::Outcome;

const CWD: &str = "f:/repo/app";

fn reason(outcome: Result<crate::bridge::codex_resolve::Lookup, Outcome>) -> String {
    match outcome {
        Err(Outcome::Failed(reason)) => reason,
        other => panic!("expected a failure, got {other:?}"),
    }
}

#[tokio::test]
async fn two_list_matches_are_ambiguous_and_name_both_ids() {
    let codex = FakeCodex::default();
    codex.on("thread/list", Ok(listed(&[("t2", Some("worker"), CWD), ("t1", Some("worker"), CWD)])));

    let reason = reason(lookup(&codex, &NameMap::default(), "worker", CWD).await);

    assert!(reason.starts_with("ambiguous"), "{reason}");
    assert!(reason.contains("t1, t2"), "{reason}");
}

#[tokio::test]
async fn a_valid_map_entry_never_hides_a_second_list_match() {
    let codex = FakeCodex::default();
    codex
        .on("thread/list", Ok(listed(&[("t2", Some("worker"), CWD)])))
        .on("thread/read", Ok(json!({"thread": {"id": "t1", "name": "worker", "cwd": CWD}})));
    let names = NameMap::default();
    names.insert("worker", CWD, "t1");

    let reason = reason(lookup(&codex, &names, "worker", CWD).await);

    assert!(reason.starts_with("ambiguous"), "{reason}");
    assert!(reason.contains("t1") && reason.contains("t2"), "{reason}");
}

#[tokio::test]
async fn a_failed_list_fails_the_lookup() {
    let codex = FakeCodex::default();
    codex.on("thread/list", Err(CallError::NotSent("the codex daemon is unreachable".into())));

    assert!(reason(lookup(&codex, &NameMap::default(), "worker", CWD).await).contains("unreachable"));
}

#[tokio::test]
async fn a_list_with_no_data_array_fails_the_lookup() {
    let codex = FakeCodex::default();
    codex.on("thread/list", Ok(json!({})));

    assert!(reason(lookup(&codex, &NameMap::default(), "worker", CWD).await).contains("no data array"));
}

#[tokio::test]
async fn a_matching_entry_with_no_id_fails_the_lookup() {
    let codex = FakeCodex::default();
    codex.on("thread/list", Ok(json!({"data": [{"name": "worker", "cwd": CWD}]})));

    assert!(reason(lookup(&codex, &NameMap::default(), "worker", CWD).await).contains("\"id\""));
}

#[tokio::test]
async fn a_map_check_that_cannot_reach_the_daemon_fails_instead_of_dropping_the_entry() {
    let codex = FakeCodex::default();
    codex.on("thread/list", Ok(listed(&[]))).on("thread/read", Err(lost("no answer")));
    let names = NameMap::default();
    names.insert("worker", CWD, "t1");

    assert!(matches!(lookup(&codex, &names, "worker", CWD).await, Err(Outcome::Failed(_))));
    codex.on("thread/list", Ok(listed(&[]))).on("thread/read", Ok(json!({"thread": {"name": "worker", "cwd": CWD}})));
    assert_eq!(lookup(&codex, &names, "worker", CWD).await, Ok(crate::bridge::codex_resolve::Lookup::Found("t1".into())));
}
