use super::*;

#[test]
fn a_known_group_resolves_to_its_sid() {
    assert_eq!(lookup_sid("Administrators").unwrap().as_deref(), Some("S-1-5-32-544"));
}
