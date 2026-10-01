use super::*;

#[test]
fn a_file_the_user_added_beside_the_skill_survives_removal() {
    let config = tempfile::tempdir().unwrap();
    write_skill(&CLAUDE, config.path(), Path::new(EXE)).unwrap();
    let note = skill_dir(config.path()).join("notes.md");
    std::fs::write(&note, "mine").unwrap();

    remove_skill(config.path()).unwrap();

    assert_eq!(std::fs::read_to_string(&note).unwrap(), "mine");
    assert!(!skill_dir(config.path()).join(SKILL_FILE).exists());
}

#[test]
fn other_skills_are_untouched_by_removal() {
    let config = tempfile::tempdir().unwrap();
    write_skill(&CLAUDE, config.path(), Path::new(EXE)).unwrap();
    let other = config.path().join("skills").join("docker");
    std::fs::create_dir_all(&other).unwrap();
    std::fs::write(other.join(SKILL_FILE), "docker").unwrap();

    remove_skill(config.path()).unwrap();

    assert_eq!(std::fs::read_to_string(other.join(SKILL_FILE)).unwrap(), "docker");
}
