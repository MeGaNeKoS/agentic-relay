use super::*;

#[test]
fn the_skill_text_is_written_under_skills_relay_in_an_existing_config_folder() {
    let config = tempfile::tempdir().unwrap();

    let change = write_skill(&CLAUDE, config.path(), Path::new(EXE)).unwrap();

    let file = config.path().join("skills").join("relay").join("SKILL.md");
    assert_eq!(change, Change::Written(file.clone()));
    assert_eq!(std::fs::read_to_string(file).unwrap(), render(&CLAUDE, Path::new(EXE)));
}

#[test]
fn each_agent_gets_its_own_text() {
    let claude = tempfile::tempdir().unwrap();
    let codex = tempfile::tempdir().unwrap();

    write_skill(&CLAUDE, claude.path(), Path::new(EXE)).unwrap();
    write_skill(&CODEX, codex.path(), Path::new(EXE)).unwrap();

    let read = |dir: &Path| std::fs::read_to_string(skill_dir(dir).join(SKILL_FILE)).unwrap();
    assert_eq!(read(claude.path()), render(&CLAUDE, Path::new(EXE)));
    assert_eq!(read(codex.path()), render(&CODEX, Path::new(EXE)));
    assert_ne!(CLAUDE.skill, CODEX.skill);
}

#[test]
fn installing_again_replaces_a_changed_skill_file() {
    let config = tempfile::tempdir().unwrap();
    write_skill(&CODEX, config.path(), Path::new(EXE)).unwrap();
    let file = skill_dir(config.path()).join(SKILL_FILE);
    std::fs::write(&file, "edited").unwrap();

    write_skill(&CODEX, config.path(), Path::new(EXE)).unwrap();

    assert_eq!(std::fs::read_to_string(file).unwrap(), render(&CODEX, Path::new(EXE)));
}
