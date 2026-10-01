use super::*;

#[test]
fn a_written_codex_skill_is_followed_by_the_daemon_restart_note() {
    let file = PathBuf::from("/home/u/.codex/skills/relay/SKILL.md");
    let text = describe(&CODEX, &Change::Written(file.clone()));
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some(format!("installed the Codex skill at {}", file.display()).as_str()));
    let note = lines.next().unwrap();
    assert!(note.contains("loads skills only when it starts"));
    assert!(note.contains("`codex app-server daemon restart`"));
    assert_eq!(lines.next(), None);
}

#[test]
fn a_written_claude_skill_has_no_note() {
    let file = PathBuf::from("/home/u/.claude/skills/relay/SKILL.md");
    let text = describe(&CLAUDE, &Change::Written(file.clone()));
    assert_eq!(text, format!("installed the Claude Code skill at {}", file.display()));
}
