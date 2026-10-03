use super::*;

fn stripped(c: char) -> Option<String> {
    clean_display_name(&format!("a{c}b"))
}

#[test]
fn double_quotes_are_removed() {
    assert_eq!(stripped('"').as_deref(), Some("ab"));
}

#[test]
fn angle_brackets_are_removed() {
    assert_eq!(stripped('<').as_deref(), Some("ab"));
    assert_eq!(stripped('>').as_deref(), Some("ab"));
}

#[test]
fn control_characters_are_removed() {
    for c in ['\u{0}', '\n', '\r', '\t', '\u{1b}', '\u{7f}', '\u{85}', '\u{9f}'] {
        assert_eq!(stripped(c).as_deref(), Some("ab"), "U+{:04X}", c as u32);
    }
}

#[test]
fn format_characters_are_removed() {
    for c in ['\u{ad}', '\u{600}', '\u{200b}', '\u{200f}', '\u{202e}', '\u{2060}', '\u{2066}', '\u{feff}', '\u{e0001}', '\u{e007f}'] {
        assert_eq!(stripped(c).as_deref(), Some("ab"), "U+{:04X}", c as u32);
    }
}

#[test]
fn line_and_paragraph_separators_are_removed() {
    assert_eq!(stripped('\u{2028}').as_deref(), Some("ab"));
    assert_eq!(stripped('\u{2029}').as_deref(), Some("ab"));
}

#[test]
fn a_character_next_to_a_stripped_range_is_kept() {
    for c in ['\u{ac}', '\u{ae}', '\u{200a}', '\u{2010}', '\u{205f}', '\u{fffc}'] {
        assert_eq!(stripped(c), Some(format!("a{c}b")), "U+{:04X}", c as u32);
    }
}
