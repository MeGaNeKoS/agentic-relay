use super::*;

#[test]
fn the_descriptor_text_grants_read_and_write_to_each_sandbox_sid() {
    let text = sddl("S-1-5-21-1", &["S-1-5-21-2".to_string(), "S-1-5-21-3".to_string()]);
    assert_eq!(text, "D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;S-1-5-21-1)(A;;GRGW;;;S-1-5-21-2)(A;;GRGW;;;S-1-5-21-3)");
}
