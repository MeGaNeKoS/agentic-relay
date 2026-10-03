use super::*;

#[test]
fn an_empty_name_is_none() {
    assert_eq!(clean_display_name(""), None);
}

#[test]
fn a_blank_name_is_none() {
    assert_eq!(clean_display_name(" \t "), None);
}

#[test]
fn a_name_made_only_of_removed_characters_is_none() {
    assert_eq!(clean_display_name("\"<>\u{0}\u{200b}\u{2028}\u{2029}"), None);
}
