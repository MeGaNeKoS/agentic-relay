use std::process::{Command, Stdio};
use std::time::Duration;

use super::*;

#[test]
fn terminating_a_running_process_ends_the_wait() {
    let mut child = Command::new("cmd").args(["/C", "pause"]).stdin(Stdio::piped()).stdout(Stdio::null()).spawn().unwrap();
    let watch = WatchedProcess::open(child.id()).unwrap().unwrap();

    watch.terminate().unwrap();

    assert!(watch.wait(Duration::from_secs(30)).unwrap());
    child.wait().unwrap();
}
