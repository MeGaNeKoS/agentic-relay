use serde_json::json;

use super::super::codex_resolve::{Lookup, NameMap, lookup};
use super::*;

const CWD: &str = "F:\\repo\\app";

fn read_of(name: &str, cwd: &str) -> Value {
    json!({"thread": {"id": "t1", "name": name, "cwd": cwd}})
}

#[tokio::test]
async fn one_exact_match_in_the_list_is_found() {
    let codex = FakeCodex::default();
    codex.on("thread/list", Ok(listed(&[("t1", Some("worker"), "f:/repo/app")])));

    let found = lookup(&codex, &NameMap::default(), "worker", CWD).await;

    assert_eq!(found, Ok(Lookup::Found("t1".into())));
    assert_eq!(codex.params_of("thread/list")[0]["searchTerm"], "worker");
}

#[tokio::test]
async fn substring_name_matches_and_other_cwds_are_ignored() {
    let codex = FakeCodex::default();
    codex.on(
        "thread/list",
        Ok(listed(&[("t2", Some("worker-2"), "f:/repo/app"), ("t3", Some("worker"), "f:/other"), ("t4", Some("Worker"), "f:/repo/app")])),
    );

    assert_eq!(lookup(&codex, &NameMap::default(), "worker", CWD).await, Ok(Lookup::Missing));
}

#[tokio::test]
async fn unnamed_threads_do_not_break_the_listing() {
    let codex = FakeCodex::default();
    codex.on("thread/list", Ok(listed(&[("t0", None, "f:/repo/app"), ("t1", Some("worker"), "f:/repo/app")])));

    assert_eq!(lookup(&codex, &NameMap::default(), "worker", CWD).await, Ok(Lookup::Found("t1".into())));
}

#[tokio::test]
async fn every_page_is_read() {
    let codex = FakeCodex::default();
    codex
        .on("thread/list", Ok(json!({"data": [], "nextCursor": "c1"})))
        .on("thread/list", Ok(listed(&[("t1", Some("worker"), "f:/repo/app")])));

    assert_eq!(lookup(&codex, &NameMap::default(), "worker", CWD).await, Ok(Lookup::Found("t1".into())));
    assert_eq!(codex.params_of("thread/list")[1]["cursor"], "c1");
}

#[tokio::test]
async fn nothing_listed_and_nothing_mapped_is_missing() {
    let codex = FakeCodex::default();
    codex.on("thread/list", Ok(listed(&[])));

    assert_eq!(lookup(&codex, &NameMap::default(), "worker", CWD).await, Ok(Lookup::Missing));
}

#[tokio::test]
async fn a_mapped_thread_the_list_has_not_caught_up_to_is_found() {
    let codex = FakeCodex::default();
    codex.on("thread/list", Ok(listed(&[]))).on("thread/read", Ok(read_of("worker", "f:/repo/app")));
    let names = NameMap::default();
    names.insert("worker", CWD, "t1");

    assert_eq!(lookup(&codex, &names, "worker", CWD).await, Ok(Lookup::Found("t1".into())));
}

#[tokio::test]
async fn the_list_still_runs_when_the_map_has_an_entry() {
    let codex = FakeCodex::default();
    codex.on("thread/list", Ok(listed(&[("t1", Some("worker"), "f:/repo/app")]))).on("thread/read", Ok(read_of("worker", "f:/repo/app")));
    let names = NameMap::default();
    names.insert("worker", CWD, "t1");

    assert_eq!(lookup(&codex, &names, "worker", CWD).await, Ok(Lookup::Found("t1".into())));
    assert_eq!(codex.params_of("thread/list").len(), 1);
}

#[tokio::test]
async fn a_mapped_thread_that_was_renamed_is_dropped() {
    let codex = FakeCodex::default();
    codex.on("thread/list", Ok(listed(&[]))).on("thread/read", Ok(read_of("renamed", "f:/repo/app")));
    let names = NameMap::default();
    names.insert("worker", CWD, "t1");

    assert_eq!(lookup(&codex, &names, "worker", CWD).await, Ok(Lookup::Missing));

    codex.on("thread/list", Ok(listed(&[])));
    assert_eq!(lookup(&codex, &names, "worker", CWD).await, Ok(Lookup::Missing));
    assert_eq!(codex.params_of("thread/read").len(), 1, "the dropped entry is not read again");
}

#[tokio::test]
async fn a_mapped_thread_that_moved_cwd_is_dropped() {
    let codex = FakeCodex::default();
    codex.on("thread/list", Ok(listed(&[]))).on("thread/read", Ok(read_of("worker", "f:/elsewhere")));
    let names = NameMap::default();
    names.insert("worker", CWD, "t1");

    assert_eq!(lookup(&codex, &names, "worker", CWD).await, Ok(Lookup::Missing));
}

#[tokio::test]
async fn a_mapped_thread_the_daemon_no_longer_has_is_dropped() {
    let codex = FakeCodex::default();
    codex.on("thread/list", Ok(listed(&[]))).on("thread/read", Err(rejected("thread not found")));
    let names = NameMap::default();
    names.insert("worker", CWD, "t1");

    assert_eq!(lookup(&codex, &names, "worker", CWD).await, Ok(Lookup::Missing));
}

#[tokio::test]
async fn the_same_thread_from_list_and_map_is_one_match() {
    let codex = FakeCodex::default();
    codex.on("thread/list", Ok(listed(&[("t1", Some("worker"), "f:/repo/app")]))).on("thread/read", Ok(read_of("worker", "f:/repo/app")));
    let names = NameMap::default();
    names.insert("worker", CWD, "t1");

    assert_eq!(lookup(&codex, &names, "worker", "f:/repo/app/").await, Ok(Lookup::Found("t1".into())));
}
