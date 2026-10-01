use super::*;

#[test]
fn a_missing_config_folder_is_skipped_and_not_created() {
    let home = tempfile::tempdir().unwrap();
    let config = home.path().join(".codex");

    let change = write_skill(&CODEX, &config, Path::new(EXE)).unwrap();

    assert_eq!(change, Change::Skipped(config.clone()));
    assert!(!config.exists());
}

#[test]
fn a_config_path_that_is_a_file_is_skipped() {
    let home = tempfile::tempdir().unwrap();
    let config = home.path().join(".claude");
    std::fs::write(&config, "not a folder").unwrap();

    assert!(matches!(write_skill(&CLAUDE, &config, Path::new(EXE)).unwrap(), Change::Skipped(_)));
}
