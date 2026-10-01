use std::process::Command;
use std::time::Duration;

use super::*;

#[test]
fn a_process_that_exits_ends_the_wait_before_the_timeout() {
    let mut child = Command::new("cmd").args(["/C", "exit", "0"]).spawn().unwrap();
    let watch = WatchedProcess::open(child.id()).unwrap().unwrap();

    assert!(watch.wait(Duration::from_secs(30)).unwrap());
    child.wait().unwrap();
}
