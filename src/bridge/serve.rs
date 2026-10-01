use std::sync::Arc;

use anyhow::Result;
use interprocess::local_socket::tokio::Stream;

use super::Bridge;
use crate::local_endpoint::{self, REQUEST_READ_TIMEOUT};
use crate::outcome::Outcome;
use crate::protocol::{Request, Response};

pub async fn handle_client(conn: Stream, bridge: Arc<Bridge>) -> Result<()> {
    let response = respond(&conn, &bridge).await;
    local_endpoint::write_json_line(&conn, &serde_json::to_string(&response)?).await
}

fn failed_before_parsing(reason: String) -> Response {
    Response::from_outcome(String::new(), Outcome::failed(reason))
}

async fn respond(conn: &Stream, bridge: &Bridge) -> Response {
    let cli_pid = match local_endpoint::peer_pid(conn) {
        Ok(pid) => pid,
        Err(e) => return failed_before_parsing(format!("{e:#}")),
    };
    let line = match local_endpoint::read_json_line(conn, REQUEST_READ_TIMEOUT).await {
        Ok(Some(line)) => line,
        Ok(None) => return failed_before_parsing("incomplete request".to_string()),
        Err(e) => return failed_before_parsing(format!("reading the request: {e:#}")),
    };
    let request: Request = match serde_json::from_str(&line) {
        Ok(request) => request,
        Err(e) => return failed_before_parsing(format!("malformed request: {e}")),
    };
    let Request::Send { message_id, caller_candidates, target, text } = request;
    let outcome = bridge.send(cli_pid, &message_id, &caller_candidates, &target, &text).await;
    Response::from_outcome(message_id, outcome)
}
