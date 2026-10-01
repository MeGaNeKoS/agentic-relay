use super::*;

#[test]
fn a_rendered_skill_runs_the_given_executable_in_every_command_line() {
    for agent in &AGENTS {
        let text = render(agent, Path::new(EXE));
        assert!(text.contains("C:/tools/relay/relay.exe send "), "{}", agent.label);
        assert!(!text.contains(EXE_PLACEHOLDER), "{}", agent.label);
        assert!(!text.contains("relay send"), "{}", agent.label);
    }
}

#[test]
fn backslashes_in_the_path_become_forward_slashes() {
    assert_eq!(command_path(Path::new(EXE)), "C:/tools/relay/relay.exe");
}

#[test]
fn rendered_command_lines_for_a_sample_path() {
    let lines: Vec<String> = AGENTS
        .iter()
        .flat_map(|agent| render(agent, Path::new(EXE)).lines().filter(|l| l.contains("relay.exe")).map(String::from).collect::<Vec<_>>())
        .collect();
    assert!(lines.iter().all(|line| line.contains("C:/tools/relay/relay.exe send ")), "{lines:#?}");
    assert!(lines.contains(&"C:/tools/relay/relay.exe send claude <session id> \"<message>\"".to_string()));
    assert!(lines.contains(&"C:/tools/relay/relay.exe send codex <name> <cwd> \"<message>\"".to_string()));
}
