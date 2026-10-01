use super::*;

#[test]
fn any_other_delete_failure_carries_the_status_and_stderr() {
    let error = delete_failure("exit code: 1", "ERROR: The system cannot find the file specified.");

    assert!(!error.is::<ElevationRequired>());
    assert_eq!(error.to_string(), "schtasks /Delete exited exit code: 1: ERROR: The system cannot find the file specified.");
}
