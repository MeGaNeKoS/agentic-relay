use super::*;

#[test]
fn a_named_sender_adds_from_name_right_after_from() {
    let frame = render("id-1", &Identity::Claude { session_id: "s1".into() }, Some("planner"), "hello");
    assert!(frame.starts_with("<cross-session-message id=\"id-1\" from=\"claude s1\" from-name=\"planner\">\n"), "{frame}");
}

#[test]
fn from_name_does_not_change_the_reply_address() {
    let named = render("id-1", &Identity::Codex { thread_id: "th".into() }, Some("planner"), "hello");
    assert!(named.contains("from=\"codex th\" from-name="), "{named}");
}
