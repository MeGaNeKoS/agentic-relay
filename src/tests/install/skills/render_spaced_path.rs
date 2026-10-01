use super::*;

#[test]
fn a_path_with_spaces_is_double_quoted_in_every_command_line() {
    let exe = Path::new("C:\\Program Files\\relay\\relay.exe");
    assert_eq!(command_path(exe), "\"C:/Program Files/relay/relay.exe\"");
    let text = render(&CODEX, exe);
    assert!(text.contains("\"C:/Program Files/relay/relay.exe\" send claude <session id>"));
    assert!(!text.contains("relay send"));
}
