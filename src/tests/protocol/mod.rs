use super::*;

#[test]
fn send_request_round_trips_through_json() {
    let request = Request::Send {
        message_id: "5b0f0a52-6c1c-4c8e-9a52-1d6e0d2f1f10".into(),
        caller_candidates: vec![
            Candidate::Claude { pid: 7, session_id: "s".into() },
            Candidate::Codex { thread_id: "t".into() },
        ],
        target: Target::CodexName { name: "n".into(), cwd: "c".into() },
        text: "hi".into(),
    };
    let line = serde_json::to_string(&request).unwrap();
    assert!(line.contains(r#""action":"send""#));
    let back: Request = serde_json::from_str(&line).unwrap();
    let Request::Send { target, caller_candidates, .. } = back;
    assert_eq!(target, Target::CodexName { name: "n".into(), cwd: "c".into() });
    assert_eq!(caller_candidates.len(), 2);
}

#[test]
fn delivered_without_a_note_omits_the_field() {
    let line = serde_json::to_string(&Response::Delivered { message_id: "m".into(), note: None }).unwrap();
    assert_eq!(line, r#"{"status":"delivered","message_id":"m"}"#);
}

#[test]
fn unknown_action_is_rejected() {
    assert!(serde_json::from_str::<Request>(r#"{"action":"interrupt"}"#).is_err());
}
