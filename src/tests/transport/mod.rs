use super::norm_cwd;

#[test]
fn normalizes_separators_case_and_trailing_slash() {
    assert_eq!(norm_cwd("F:\\Programming\\Rust\\"), "f:/programming/rust");
    assert_eq!(norm_cwd("f:/programming/rust"), "f:/programming/rust");
}
