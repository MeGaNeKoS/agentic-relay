use super::*;

#[test]
fn the_environment_value_replaces_the_default_folder() {
    let dir = config_dir(&CLAUDE, Some(OsString::from("/custom/claude")), Path::new("/home/u"));
    assert_eq!(dir, PathBuf::from("/custom/claude"));
}

#[test]
fn each_agent_reads_its_own_variable() {
    assert_eq!(CLAUDE.config_env, "CLAUDE_CONFIG_DIR");
    assert_eq!(CODEX.config_env, "CODEX_HOME");
}
