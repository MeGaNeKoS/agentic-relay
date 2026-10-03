use super::*;

#[test]
fn a_name_of_exactly_64_characters_is_kept_whole() {
    let name = "x".repeat(64);
    assert_eq!(clean_display_name(&name), Some(name));
}

#[test]
fn a_longer_name_keeps_64_characters_and_gains_an_ellipsis() {
    let cleaned = clean_display_name(&"x".repeat(65)).unwrap();
    assert_eq!(cleaned, format!("{}\u{2026}", "x".repeat(64)));
}

#[test]
fn characters_are_counted_not_bytes() {
    let cleaned = clean_display_name(&"\u{4e2d}".repeat(70)).unwrap();
    assert_eq!(cleaned.chars().count(), 65);
    assert!(cleaned.ends_with('\u{2026}'));
}

#[test]
fn removed_characters_do_not_count_toward_the_limit() {
    let name = format!("{}\"<>", "x".repeat(64));
    assert_eq!(clean_display_name(&name), Some("x".repeat(64)));
}
