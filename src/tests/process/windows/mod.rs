mod watched_process_exited;
mod watched_process_gone;
mod watched_process_running;
mod watched_process_terminated;

use super::*;

#[test]
fn own_process_has_a_start_time_and_a_parent() {
    let pid = std::process::id();
    assert!(start_time(pid).unwrap().is_some());
    assert!(process_table().unwrap().contains_key(&pid));
}

#[test]
fn a_pid_that_does_not_exist_has_no_start_time() {
    assert_eq!(start_time(0x7fff_fff0).unwrap(), None);
}

#[test]
fn ancestry_of_own_process_starts_with_itself() {
    let chain = crate::process::ancestry(std::process::id()).unwrap();
    assert_eq!(chain[0].pid, std::process::id());
}

#[test]
fn a_described_chain_names_every_step_and_where_it_ends() {
    let chain = crate::process::ancestry(std::process::id()).unwrap();
    let text = crate::process::describe_chain(&chain);
    assert!(text.contains(&format!("pid {} parent", std::process::id())), "{text}");
    assert!(text.contains("image relay"), "{text}");
    assert!(text.contains("chain ends:"), "{text}");
}
