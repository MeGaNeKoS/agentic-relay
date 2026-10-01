use super::*;

#[test]
fn a_skipped_agent_is_reported_with_its_missing_folder() {
    let line = describe(&CODEX, &Change::Skipped(PathBuf::from("/home/u/.codex")));
    assert_eq!(line, format!("skipped the Codex skill: {} does not exist", Path::new("/home/u/.codex").display()));
}
