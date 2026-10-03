use super::*;

#[test]
fn an_unnamed_sender_leaves_the_envelope_without_from_name() {
    let frame = render("id-1", &Identity::Claude { session_id: "s1".into() }, None, "hello");
    assert_eq!(frame, "<cross-session-message id=\"id-1\" from=\"claude s1\">\nhello\n</cross-session-message>");
}
