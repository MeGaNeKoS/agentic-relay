use super::*;

#[test]
fn a_plain_name_is_kept() {
    assert_eq!(clean_display_name("planner").as_deref(), Some("planner"));
}

#[test]
fn inner_spaces_and_non_ascii_letters_are_kept() {
    assert_eq!(clean_display_name("my worker \u{e9}\u{4e2d}").as_deref(), Some("my worker \u{e9}\u{4e2d}"));
}

#[test]
fn surrounding_whitespace_is_trimmed() {
    assert_eq!(clean_display_name("  planner \t").as_deref(), Some("planner"));
}
