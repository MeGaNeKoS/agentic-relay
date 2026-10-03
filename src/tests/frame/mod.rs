use super::*;

mod from_name;
mod unnamed;

#[test]
fn starts_with_the_message_id_and_carries_the_reply_address() {
    let frame = render("id-1", &Identity::Codex { thread_id: "th".into() }, None, "hello");
    assert!(frame.starts_with("<cross-session-message id=\"id-1\" from=\"codex th\">"));
    assert!(frame.contains("\nhello\n"));
}

#[test]
fn different_ids_give_different_frames_for_the_same_text() {
    let sender = Identity::Claude { session_id: "s".into() };
    assert_ne!(render("a", &sender, None, "x"), render("b", &sender, None, "x"));
}
