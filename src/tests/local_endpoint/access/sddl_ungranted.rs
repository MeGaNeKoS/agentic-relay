use super::*;

#[test]
fn without_sandbox_sids_only_the_user_and_administrators_are_granted() {
    assert_eq!(sddl("S-1-5-21-1", &[]), "D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;S-1-5-21-1)");
}
