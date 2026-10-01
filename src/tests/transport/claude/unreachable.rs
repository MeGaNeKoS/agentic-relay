use super::super::deliver_with;
use super::*;
use crate::outcome::Outcome;

fn reason(outcome: Outcome) -> String {
    match outcome {
        Outcome::Failed(reason) => reason,
        other => panic!("expected a failure, got {other:?}"),
    }
}

#[tokio::test]
async fn no_session_file_for_the_id_is_not_reachable() {
    let dir = tempfile::tempdir().unwrap();
    write_session(dir.path(), 9, "other", 90, "pipe");

    let reason = reason(deliver_with(dir.path(), "s1", FRAME, alive_except(&[])).await);

    assert!(reason.starts_with("not reachable"), "{reason}");
}

#[tokio::test]
async fn a_missing_sessions_directory_is_not_reachable() {
    let dir = tempfile::tempdir().unwrap();

    let reason = reason(deliver_with(&dir.path().join("absent"), "s1", FRAME, alive_except(&[])).await);

    assert!(reason.starts_with("not reachable"), "{reason}");
}

#[tokio::test]
async fn a_file_whose_process_has_exited_is_not_reachable() {
    let dir = tempfile::tempdir().unwrap();
    write_session(dir.path(), 7, "s1", 70, "pipe");

    let reason = reason(deliver_with(dir.path(), "s1", FRAME, |_| Ok(None)).await);

    assert!(reason.starts_with("not reachable"), "{reason}");
}

#[tokio::test]
async fn a_file_whose_pid_was_reused_by_another_process_is_not_reachable() {
    let dir = tempfile::tempdir().unwrap();
    write_session(dir.path(), 7, "s1", 70, "pipe");

    let reason = reason(deliver_with(dir.path(), "s1", FRAME, alive_except(&[7])).await);

    assert!(reason.starts_with("not reachable"), "{reason}");
}

#[tokio::test]
async fn two_live_files_for_one_id_are_ambiguous_and_name_both_pids() {
    let dir = tempfile::tempdir().unwrap();
    write_session(dir.path(), 7, "s1", 70, "pipe-a");
    write_session(dir.path(), 8, "s1", 80, "pipe-b");

    let reason = reason(deliver_with(dir.path(), "s1", FRAME, alive_except(&[])).await);

    assert!(reason.starts_with("ambiguous"), "{reason}");
    assert!(reason.contains('7') && reason.contains('8'), "{reason}");
}

#[tokio::test]
async fn a_live_session_whose_peer_key_is_missing_fails() {
    let dir = tempfile::tempdir().unwrap();
    write_session(dir.path(), 7, "s1", 70, "pipe");

    assert!(matches!(deliver_with(dir.path(), "s1", FRAME, alive_except(&[])).await, Outcome::Failed(_)));
}

#[tokio::test]
async fn a_peer_key_for_a_different_process_start_fails() {
    let dir = tempfile::tempdir().unwrap();
    write_session(dir.path(), 7, "s1", 70, "pipe");
    write_key(dir.path(), 7, "pipe", "tok", 71);

    let reason = reason(deliver_with(dir.path(), "s1", FRAME, alive_except(&[])).await);

    assert!(reason.contains("does not match"), "{reason}");
}

#[tokio::test]
async fn an_inbox_pipe_that_cannot_be_opened_fails_without_a_write() {
    let dir = tempfile::tempdir().unwrap();
    let pipe = unique_pipe_name();
    write_session(dir.path(), 7, "s1", 70, &pipe);
    write_key(dir.path(), 7, &pipe, "tok", 70);

    let reason = reason(deliver_with(dir.path(), "s1", FRAME, alive_except(&[])).await);

    assert!(reason.contains("opening the session's inbox pipe"), "{reason}");
}
