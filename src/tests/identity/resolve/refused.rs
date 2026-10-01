use super::*;
use crate::identity::resolve::select_candidate;

fn refusal(candidates: &[Candidate], chain: &[Ancestor], env: &FakeEnvironment) -> String {
    select_candidate(candidates, chain, env).unwrap_err().0
}

#[test]
fn no_candidates_is_refused() {
    let env = FakeEnvironment { chain: vec![], sessions: vec![], daemon: None };
    assert!(refusal(&[], &[ancestor(10, 100)], &env).starts_with("no caller identity"));
}

#[test]
fn claude_process_outside_the_chain_is_refused() {
    let env = FakeEnvironment { chain: vec![], sessions: vec![session(50, "s1", 500)], daemon: None };
    let reason = refusal(&[claude(50, "s1")], &[ancestor(10, 100)], &env);
    assert!(reason.contains("not an ancestor"), "{reason}");
}

#[test]
fn reused_pid_with_a_different_start_time_is_refused() {
    let env = FakeEnvironment { chain: vec![], sessions: vec![session(50, "s1", 500)], daemon: None };
    let reason = refusal(&[claude(50, "s1")], &[ancestor(10, 100), ancestor(50, 999)], &env);
    assert!(reason.contains("not an ancestor"), "{reason}");
}

#[test]
fn session_id_that_differs_from_the_session_file_is_refused() {
    let env = FakeEnvironment { chain: vec![], sessions: vec![session(50, "other", 500)], daemon: None };
    let reason = refusal(&[claude(50, "s1")], &[ancestor(50, 500)], &env);
    assert!(reason.contains("different session"), "{reason}");
}

#[test]
fn missing_session_file_is_refused() {
    let env = FakeEnvironment { chain: vec![], sessions: vec![], daemon: None };
    let reason = refusal(&[claude(50, "s1")], &[ancestor(50, 500)], &env);
    assert!(reason.contains("no session file"), "{reason}");
}

#[test]
fn codex_candidate_without_a_trusted_daemon_is_refused() {
    let env = FakeEnvironment { chain: vec![], sessions: vec![], daemon: None };
    let reason = refusal(&[codex("t1")], &[ancestor(60, 600)], &env);
    assert!(reason.contains("no live daemon"), "{reason}");
}

#[test]
fn codex_candidate_whose_daemon_is_not_an_ancestor_is_refused() {
    let env = FakeEnvironment { chain: vec![], sessions: vec![], daemon: Some(ancestor(60, 600)) };
    let reason = refusal(&[codex("t1")], &[ancestor(10, 100)], &env);
    assert!(reason.contains("Codex daemon is not an ancestor"), "{reason}");
}

#[test]
fn daemon_pid_reused_with_a_different_start_time_is_refused() {
    let env = FakeEnvironment { chain: vec![], sessions: vec![], daemon: Some(ancestor(60, 600)) };
    let reason = refusal(&[codex("t1")], &[ancestor(10, 100), ancestor(60, 601)], &env);
    assert!(reason.contains("Codex daemon is not an ancestor"), "{reason}");
}
