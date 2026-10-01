mod delivery;
mod session_files;
mod unreachable;

use std::path::Path;

use serde_json::json;
use tokio::io::{AsyncBufReadExt, BufReader};

use super::key_hash;

pub(super) const FRAME: &str = "<cross-session-message id=\"m1\" from=\"codex t1\">\nhello\n</cross-session-message>";

pub(super) fn write_session(dir: &Path, pid: u32, session_id: &str, proc_start: u64, socket_path: &str) {
    let session = json!({
        "pid": pid,
        "sessionId": session_id,
        "procStart": proc_start.to_string(),
        "pidDomain": "win32:test",
        "messagingSocketPath": socket_path,
    });
    std::fs::write(dir.join(format!("{pid}.json")), session.to_string()).unwrap();
}

pub(super) fn write_key(dir: &Path, pid: u32, socket_path: &str, token: &str, proc_start: u64) {
    let key = json!({"peerToken": token, "procStartFt": proc_start.to_string(), "pidDomain": "win32:test"});
    std::fs::write(dir.join(format!("{pid}.{}.key", key_hash(socket_path))), key.to_string()).unwrap();
}

pub(super) fn alive_except(stale: &'static [u32]) -> impl Fn(u32) -> anyhow::Result<Option<u64>> {
    move |pid| Ok(if stale.contains(&pid) { Some(1) } else { Some(u64::from(pid) * 10) })
}

pub(super) fn unique_pipe_name() -> String {
    let id = uuid::Uuid::new_v4();
    if cfg!(windows) { format!(r"\\.\pipe\relay-test-{id}") } else { std::env::temp_dir().join(format!("relay-test-{id}.sock")).to_string_lossy().into_owned() }
}

#[cfg(windows)]
pub(super) fn serve_once(path: &str) -> tokio::task::JoinHandle<Vec<String>> {
    use tokio::net::windows::named_pipe::ServerOptions;

    let server = ServerOptions::new().first_pipe_instance(true).create(path).unwrap();
    tokio::spawn(async move {
        server.connect().await.unwrap();
        read_lines(server).await
    })
}

#[cfg(not(windows))]
pub(super) fn serve_once(path: &str) -> tokio::task::JoinHandle<Vec<String>> {
    let listener = tokio::net::UnixListener::bind(path).unwrap();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        read_lines(stream).await
    })
}

async fn read_lines(stream: impl tokio::io::AsyncRead + Unpin) -> Vec<String> {
    let mut lines = BufReader::new(stream).lines();
    let mut out = Vec::new();
    while let Some(line) = lines.next_line().await.unwrap() {
        out.push(line);
    }
    out
}
