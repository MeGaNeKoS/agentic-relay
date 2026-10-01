use super::*;

#[test]
fn the_current_user_has_a_sid_and_the_real_descriptor_builds() {
    assert!(current_user_sid().unwrap().starts_with("S-1-5-"));
    security_descriptor().unwrap();
}
