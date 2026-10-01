use std::process::{Command, Stdio};
use std::time::Duration;

use super::*;

#[test]
fn a_process_that_keeps_running_times_out_the_wait() {
    let mut child = Command::new("cmd").args(["/C", "pause"]).stdin(Stdio::piped()).stdout(Stdio::null()).spawn().unwrap();
    let watch = WatchedProcess::open(child.id()).unwrap().unwrap();

    let exited = watch.wait(Duration::from_millis(200)).unwrap();

    child.kill().unwrap();
    child.wait().unwrap();
    assert!(!exited);
}
