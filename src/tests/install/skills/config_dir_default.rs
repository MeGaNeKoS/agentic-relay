use super::*;

#[test]
fn without_an_environment_value_the_default_folder_under_home_is_used() {
    assert_eq!(config_dir(&CLAUDE, None, Path::new("/home/u")), Path::new("/home/u").join(".claude"));
    assert_eq!(config_dir(&CODEX, None, Path::new("/home/u")), Path::new("/home/u").join(".codex"));
}

#[test]
fn an_empty_environment_value_counts_as_unset() {
    assert_eq!(config_dir(&CODEX, Some(OsString::new()), Path::new("/home/u")), Path::new("/home/u").join(".codex"));
}
