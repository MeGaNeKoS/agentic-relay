use super::*;

#[test]
fn a_name_that_does_not_exist_resolves_to_none() {
    assert_eq!(lookup_sid("NoSuchGroupRelayTest").unwrap(), None);
}
