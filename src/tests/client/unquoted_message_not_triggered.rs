use super::*;

#[test]
fn a_thread_id_and_one_quoted_message_is_a_by_id_send() {
    let (target, text) = parse_send_args(&args(&["codex", "01a0f7b8-c4f5-7cb2-8fe8-9d67e4d0b2da", "hello there"])).unwrap();
    assert_eq!(target, Target::CodexId { thread_id: "01a0f7b8-c4f5-7cb2-8fe8-9d67e4d0b2da".into() });
    assert_eq!(text, "hello there");
}

#[test]
fn a_name_that_is_not_a_thread_id_still_takes_a_cwd_and_a_message() {
    let (target, _) = parse_send_args(&args(&["codex", "worker", "F:/repo", "hello"])).unwrap();
    assert_eq!(target, Target::CodexName { name: "worker".into(), cwd: "F:/repo".into() });
}
