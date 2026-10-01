use super::*;

#[test]
fn removal_deletes_the_skill_file_and_its_empty_folder() {
    let config = tempfile::tempdir().unwrap();
    write_skill(&CLAUDE, config.path(), Path::new(EXE)).unwrap();

    let change = remove_skill(config.path()).unwrap();

    assert_eq!(change, Change::Removed(skill_dir(config.path()).join(SKILL_FILE)));
    assert!(!skill_dir(config.path()).exists());
}

#[test]
fn removal_leaves_the_skills_folder_itself_in_place() {
    let config = tempfile::tempdir().unwrap();
    write_skill(&CODEX, config.path(), Path::new(EXE)).unwrap();

    remove_skill(config.path()).unwrap();

    assert!(config.path().join("skills").is_dir());
}
