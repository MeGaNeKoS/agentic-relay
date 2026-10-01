use super::super::session::{list_sessions, read_session};
use super::*;

#[test]
fn only_pid_named_json_files_are_sessions() {
    let dir = tempfile::tempdir().unwrap();
    write_session(dir.path(), 7, "s1", 70, "pipe");
    write_key(dir.path(), 7, "pipe", "tok", 70);
    std::fs::write(dir.path().join("notes.json"), "{}").unwrap();

    let sessions = list_sessions(dir.path()).unwrap();

    assert_eq!(sessions.len(), 1);
    assert_eq!((sessions[0].pid, sessions[0].session_id.as_str(), sessions[0].proc_start), (7, "s1", 70));
}

#[test]
fn a_corrupt_file_is_skipped_and_the_rest_still_list() {
    let dir = tempfile::tempdir().unwrap();
    write_session(dir.path(), 7, "s1", 70, "pipe");
    std::fs::write(dir.path().join("8.json"), "{not json").unwrap();

    assert_eq!(list_sessions(dir.path()).unwrap().len(), 1);
}

#[test]
fn a_non_numeric_proc_start_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("8.json"),
        r#"{"pid":8,"sessionId":"s","procStart":"soon","pidDomain":"d","messagingSocketPath":"p"}"#,
    )
    .unwrap();

    assert!(list_sessions(dir.path()).unwrap().is_empty());
}

#[test]
fn read_session_returns_none_for_a_missing_file_and_an_error_for_a_corrupt_one() {
    let dir = tempfile::tempdir().unwrap();
    assert!(read_session(dir.path(), 7).unwrap().is_none());
    std::fs::write(dir.path().join("7.json"), "{not json").unwrap();
    assert!(read_session(dir.path(), 7).is_err());
}
