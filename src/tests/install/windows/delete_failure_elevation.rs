use super::*;

#[test]
fn an_access_denied_delete_is_the_elevation_refusal() {
    let error = delete_failure("exit code: 1", "ERROR: Access is denied.");

    assert!(error.is::<ElevationRequired>());
    assert!(error.to_string().starts_with("removing the logon task needs an elevated"));
}
