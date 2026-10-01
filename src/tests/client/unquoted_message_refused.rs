use super::*;

#[test]
fn a_thread_id_followed_by_two_more_arguments_is_refused_with_the_quoting_hint() {
    let err = parse_send_args(&args(&["codex", "01a0f7b8-c4f5-7cb2-8fe8-9d67e4d0b2da", "extra", "words"])).unwrap_err().to_string();
    assert!(err.contains("quote the whole message"), "{err}");
}
