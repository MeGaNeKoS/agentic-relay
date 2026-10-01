mod response_timeout;
mod unquoted_message_not_triggered;
mod unquoted_message_refused;

use super::*;

fn args(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

#[test]
fn claude_address_takes_a_session_id_and_a_message() {
    let (target, text) = parse_send_args(&args(&["claude", "sid", "hello"])).unwrap();
    assert_eq!(target, Target::Claude { session_id: "sid".into() });
    assert_eq!(text, "hello");
}

#[test]
fn codex_id_address_has_two_parts() {
    let (target, _) = parse_send_args(&args(&["codex", "tid", "hello"])).unwrap();
    assert_eq!(target, Target::CodexId { thread_id: "tid".into() });
}

#[test]
fn codex_name_address_has_a_name_and_a_cwd() {
    let (target, text) = parse_send_args(&args(&["codex", "worker", "F:/repo", "hello"])).unwrap();
    assert_eq!(target, Target::CodexName { name: "worker".into(), cwd: "F:/repo".into() });
    assert_eq!(text, "hello");
}

#[test]
fn unknown_kinds_and_wrong_arities_are_rejected() {
    assert!(parse_send_args(&args(&["gemini", "x", "hello"])).is_err());
    assert!(parse_send_args(&args(&["claude", "a", "b", "c"])).is_err());
    assert!(parse_send_args(&args(&["codex", "hello"])).is_err());
}

#[test]
fn every_named_candidate_is_collected() {
    let candidates = collect_candidates(Some("12".into()), Some("sid".into()), Some("tid".into())).unwrap();
    assert_eq!(
        candidates,
        vec![Candidate::Claude { pid: 12, session_id: "sid".into() }, Candidate::Codex { thread_id: "tid".into() }]
    );
}

#[test]
fn no_environment_identity_is_an_error() {
    assert!(collect_candidates(None, None, None).is_err());
}

#[test]
fn a_half_set_claude_environment_is_an_error() {
    assert!(collect_candidates(Some("12".into()), None, None).is_err());
    assert!(collect_candidates(None, Some("sid".into()), Some("tid".into())).is_err());
}

#[test]
fn a_non_numeric_claude_pid_is_an_error() {
    assert!(collect_candidates(Some("abc".into()), Some("sid".into()), None).is_err());
}

#[test]
fn only_delivered_exits_zero() {
    let delivered = Response::Delivered { message_id: "m".into(), note: None };
    let failed = Response::Failed { message_id: "m".into(), reason: "r".into() };
    let unknown = Response::MayNotHaveLanded { message_id: "m".into(), reason: "r".into() };
    assert_eq!(render(&delivered).1, ExitCode::SUCCESS);
    assert_eq!(render(&failed).1, ExitCode::from(1));
    assert_eq!(render(&unknown).1, ExitCode::from(2));
    assert!(render(&unknown).0.contains("message id m"));
}
