use super::*;
use crate::local_endpoint::{bind_name, endpoint_name_for};

#[tokio::test]
async fn a_bound_endpoint_reports_the_pid_of_the_process_serving_it() {
    let printname = format!("relay-test-{}.sock", uuid::Uuid::new_v4());
    let _listener = bind_name(endpoint_name_for(&printname).unwrap()).unwrap();

    assert_eq!(server_pid_of(&printname).unwrap(), Some(std::process::id()));
}
