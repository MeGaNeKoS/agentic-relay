use serde_json::Value;

use super::super::deliver_with;
use super::*;
use crate::outcome::Outcome;

#[tokio::test]
async fn one_live_session_gets_the_auth_frame_then_the_message_frame() {
    let dir = tempfile::tempdir().unwrap();
    let pipe = unique_pipe_name();
    write_session(dir.path(), 7, "s1", 70, &pipe);
    write_key(dir.path(), 7, &pipe, "tok", 70);
    let inbox = serve_once(&pipe);

    let outcome = deliver_with(dir.path(), "s1", FRAME, alive_except(&[])).await;

    assert_eq!(outcome, Outcome::delivered());
    let lines = inbox.await.unwrap();
    assert_eq!(lines.len(), 2);
    let auth: Value = serde_json::from_str(&lines[0]).unwrap();
    assert_eq!((auth["type"].as_str(), auth["token"].as_str()), (Some("auth"), Some("tok")));
    let user: Value = serde_json::from_str(&lines[1]).unwrap();
    assert_eq!(user["type"], "user");
    assert_eq!(user["message"]["role"], "user");
    assert_eq!(user["message"]["content"], FRAME);
}

#[tokio::test]
async fn a_stale_file_for_the_same_session_does_not_make_the_live_one_ambiguous() {
    let dir = tempfile::tempdir().unwrap();
    let pipe = unique_pipe_name();
    write_session(dir.path(), 7, "s1", 70, &pipe);
    write_key(dir.path(), 7, &pipe, "tok", 70);
    write_session(dir.path(), 8, "s1", 80, &unique_pipe_name());
    let inbox = serve_once(&pipe);

    let outcome = deliver_with(dir.path(), "s1", FRAME, alive_except(&[8])).await;

    assert_eq!(outcome, Outcome::delivered());
    assert_eq!(inbox.await.unwrap().len(), 2);
}

#[tokio::test]
async fn sessions_with_other_ids_are_left_alone() {
    let dir = tempfile::tempdir().unwrap();
    let pipe = unique_pipe_name();
    write_session(dir.path(), 7, "s1", 70, &pipe);
    write_key(dir.path(), 7, &pipe, "tok", 70);
    write_session(dir.path(), 9, "s2", 90, &unique_pipe_name());
    let inbox = serve_once(&pipe);

    assert_eq!(deliver_with(dir.path(), "s1", FRAME, alive_except(&[])).await, Outcome::delivered());
    inbox.await.unwrap();
}
