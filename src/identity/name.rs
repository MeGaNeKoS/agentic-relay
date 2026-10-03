const MAX_CHARS: usize = 64;

const FORMAT_RANGES: &[(u32, u32)] = &[
    (0x00AD, 0x00AD),
    (0x0600, 0x0605),
    (0x061C, 0x061C),
    (0x06DD, 0x06DD),
    (0x070F, 0x070F),
    (0x0890, 0x0891),
    (0x08E2, 0x08E2),
    (0x180E, 0x180E),
    (0x200B, 0x200F),
    (0x202A, 0x202E),
    (0x2060, 0x2064),
    (0x2066, 0x206F),
    (0xFEFF, 0xFEFF),
    (0xFFF9, 0xFFFB),
    (0x110BD, 0x110BD),
    (0x110CD, 0x110CD),
    (0x13430, 0x1343F),
    (0x1BCA0, 0x1BCA3),
    (0x1D173, 0x1D17A),
    (0xE0001, 0xE0001),
    (0xE0020, 0xE007F),
];

fn is_stripped(c: char) -> bool {
    let code = c as u32;
    matches!(c, '"' | '<' | '>' | '\u{2028}' | '\u{2029}') || c.is_control() || FORMAT_RANGES.iter().any(|&(low, high)| (low..=high).contains(&code))
}

pub fn clean_display_name(raw: &str) -> Option<String> {
    let stripped: String = raw.chars().filter(|&c| !is_stripped(c)).collect();
    let trimmed = stripped.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.chars().count() > MAX_CHARS {
        let mut kept: String = trimmed.chars().take(MAX_CHARS).collect();
        kept.push('\u{2026}');
        return Some(kept);
    }
    Some(trimmed.to_string())
}

#[cfg(test)]
#[path = "../tests/identity/name/mod.rs"]
mod tests;
