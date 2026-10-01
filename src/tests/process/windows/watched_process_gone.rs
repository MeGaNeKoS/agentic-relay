use super::*;

#[test]
fn a_pid_that_does_not_exist_has_nothing_to_watch() {
    assert!(WatchedProcess::open(0x7fff_fff0).unwrap().is_none());
}
