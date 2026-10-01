use std::time::Duration;

use super::*;

fn unique_name() -> String {
    format!("relay-test-{}.sock", uuid::Uuid::new_v4())
}

async fn connected_pair() -> (Stream, Stream) {
    let printname = unique_name();
    let listener = bind_name(endpoint_name_for(&printname).unwrap()).unwrap();
    let client = Stream::connect(endpoint_name_for(&printname).unwrap()).await.unwrap();
    let server = listener.accept().await.unwrap();
    (server, client)
}

#[tokio::test]
async fn the_server_reads_the_connecting_process_id_from_the_endpoint() {
    let (server, _client) = connected_pair().await;

    assert_eq!(peer_pid(&server).unwrap(), std::process::id());
}

#[tokio::test]
async fn a_line_round_trips_with_its_newline_stripped_of_nothing() {
    let (server, client) = connected_pair().await;

    write_json_line(&client, r#"{"a":1}"#).await.unwrap();

    let line = read_json_line(&server, Duration::from_secs(5)).await.unwrap().unwrap();
    assert_eq!(line, "{\"a\":1}\n");
}

#[tokio::test]
async fn a_connection_that_closes_before_a_full_line_reads_as_none() {
    let (server, client) = connected_pair().await;
    let mut raw = &client;
    raw.write_all(b"{\"partial\":").await.unwrap();
    drop(client);

    assert_eq!(read_json_line(&server, Duration::from_secs(5)).await.unwrap(), None);
}

#[tokio::test]
async fn a_client_that_sends_nothing_times_out() {
    let (server, _client) = connected_pair().await;

    assert!(read_json_line(&server, Duration::from_millis(100)).await.is_err());
}

#[tokio::test]
async fn binding_a_name_that_is_in_use_is_refused() {
    let printname = unique_name();
    let _first = bind_name(endpoint_name_for(&printname).unwrap()).unwrap();

    let error = bind_name(endpoint_name_for(&printname).unwrap()).err().unwrap();

    assert!(error.to_string().contains("already running"), "{error}");
}
