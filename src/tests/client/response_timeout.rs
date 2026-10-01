use super::*;

#[test]
fn the_client_outwaits_a_send_whose_every_daemon_call_takes_the_full_call_wait() {
    let connect = CALL_TIMEOUT + Duration::from_secs(30);
    let seven_calls = CALL_TIMEOUT * 7;
    let turn_started_wait = Duration::from_secs(30);
    assert!(RESPONSE_TIMEOUT >= connect + seven_calls + turn_started_wait);
}
