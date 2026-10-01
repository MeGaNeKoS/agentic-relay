use super::*;

#[test]
fn each_embedded_skill_is_named_relay_and_has_a_description() {
    for agent in &AGENTS {
        assert!(agent.skill.starts_with("---\nname: relay\ndescription: "), "{}", agent.label);
    }
}

#[test]
fn the_claude_skill_sends_claude_sessions_to_sendmessage_and_the_codex_skill_does_not() {
    assert!(CLAUDE.skill.contains("SendMessage"));
    assert!(!CODEX.skill.contains("SendMessage"));
}

#[test]
fn every_command_line_in_an_embedded_skill_is_a_placeholder_not_a_bare_relay() {
    for agent in &AGENTS {
        assert!(!agent.skill.contains("relay send"), "{}", agent.label);
        assert!(agent.skill.contains("@RELAY@ send "), "{}", agent.label);
    }
}
