use super::*;
use crate::identity::resolve::select_candidate;

#[test]
fn claude_candidate_is_kept_when_its_process_is_an_ancestor() {
    let env = FakeEnvironment { chain: vec![], sessions: vec![session(50, "s1", 500)], daemon: None };
    let chain = [ancestor(10, 100), ancestor(20, 200), ancestor(50, 500)];
    assert_eq!(select_candidate(&[claude(50, "s1")], &chain, &env).unwrap().candidate, claude(50, "s1"));
}

#[test]
fn codex_candidate_is_kept_when_the_daemon_is_an_ancestor() {
    let env = FakeEnvironment { chain: vec![], sessions: vec![], daemon: Some(ancestor(60, 600)) };
    let chain = [ancestor(10, 100), ancestor(30, 300), ancestor(60, 600)];
    assert_eq!(select_candidate(&[codex("t1")], &chain, &env).unwrap().candidate, codex("t1"));
}

#[test]
fn the_nearest_ancestor_wins_when_both_kinds_qualify() {
    let env = FakeEnvironment { chain: vec![], sessions: vec![session(50, "s1", 500)], daemon: Some(ancestor(60, 600)) };
    let claude_nearer = [ancestor(10, 100), ancestor(50, 500), ancestor(60, 600)];
    let codex_nearer = [ancestor(10, 100), ancestor(60, 600), ancestor(50, 500)];
    let both = [claude(50, "s1"), codex("t1")];
    assert_eq!(select_candidate(&both, &claude_nearer, &env).unwrap().candidate, claude(50, "s1"));
    assert_eq!(select_candidate(&both, &codex_nearer, &env).unwrap().candidate, codex("t1"));
}

#[test]
fn a_candidate_that_is_not_an_ancestor_does_not_hide_one_that_is() {
    let env = FakeEnvironment { chain: vec![], sessions: vec![session(50, "s1", 500)], daemon: Some(ancestor(60, 600)) };
    let chain = [ancestor(10, 100), ancestor(60, 600)];
    let both = [claude(50, "s1"), codex("t1")];
    assert_eq!(select_candidate(&both, &chain, &env).unwrap().candidate, codex("t1"));
}
