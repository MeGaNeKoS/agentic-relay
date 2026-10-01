use super::*;

#[test]
fn an_endpoint_nobody_listens_on_has_no_server_pid() {
    let printname = format!("relay-test-{}.sock", uuid::Uuid::new_v4());

    assert_eq!(server_pid_of(&printname).unwrap(), None);
}
