use super::*;

#[test]
fn removing_with_no_skill_installed_changes_nothing_and_succeeds() {
    let config = tempfile::tempdir().unwrap();

    let change = remove_skill(config.path()).unwrap();

    assert_eq!(change, Change::Absent(skill_dir(config.path()).join(SKILL_FILE)));
}

#[test]
fn removing_from_a_missing_config_folder_succeeds() {
    let config = tempfile::tempdir().unwrap();
    let gone = config.path().join("missing");

    assert!(matches!(remove_skill(&gone).unwrap(), Change::Absent(_)));
}
