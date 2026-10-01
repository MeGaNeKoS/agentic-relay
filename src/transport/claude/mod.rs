mod pipe;
mod session;

pub use session::{SessionFile, read_session, sessions_dir};

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::outcome::Outcome;
use crate::process;

#[derive(Debug, Deserialize)]
struct PeerKey {
    #[serde(rename = "peerToken")]
    peer_token: String,
    #[serde(rename = "procStartFt")]
    proc_start_ft: String,
    #[serde(rename = "pidDomain")]
    pid_domain: String,
}

fn key_hash(messaging_socket_path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(messaging_socket_path.to_lowercase().as_bytes());
    hex::encode(hasher.finalize())
}

fn read_peer_key(dir: &Path, session: &SessionFile) -> Result<PeerKey> {
    let path = dir.join(format!("{}.{}.key", session.pid, key_hash(&session.messaging_socket_path)));
    let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", path.display()))
}

#[derive(Serialize)]
struct AuthFrame<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    token: &'a str,
}

#[derive(Serialize)]
struct UserFrame<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    message: UserMessage<'a>,
}

#[derive(Serialize)]
struct UserMessage<'a> {
    role: &'static str,
    content: &'a str,
}

fn live_sessions(
    files: Vec<SessionFile>,
    session_id: &str,
    start_time: impl Fn(u32) -> Result<Option<u64>>,
) -> Result<Vec<SessionFile>> {
    let mut live = Vec::new();
    for file in files.into_iter().filter(|f| f.session_id == session_id) {
        if start_time(file.pid)? == Some(file.proc_start) {
            live.push(file);
        }
    }
    Ok(live)
}

pub async fn deliver(sessions_dir: &Path, session_id: &str, frame: &str) -> Outcome {
    deliver_with(sessions_dir, session_id, frame, process::start_time).await
}

async fn deliver_with(
    sessions_dir: &Path,
    session_id: &str,
    frame: &str,
    start_time: impl Fn(u32) -> Result<Option<u64>>,
) -> Outcome {
    match deliver_inner(sessions_dir, session_id, frame, start_time).await {
        Ok(outcome) => outcome,
        Err(e) => Outcome::failed(format!("{e:#}")),
    }
}

async fn deliver_inner(
    sessions_dir: &Path,
    session_id: &str,
    frame: &str,
    start_time: impl Fn(u32) -> Result<Option<u64>>,
) -> Result<Outcome> {
    let files = session::list_sessions(sessions_dir)?;
    let mut live = live_sessions(files, session_id, start_time)?;
    let session = match live.len() {
        0 => return Ok(Outcome::failed(format!("not reachable: no live Claude Code session with id {session_id}"))),
        1 => live.remove(0),
        _ => {
            let pids: Vec<String> = live.iter().map(|s| s.pid.to_string()).collect();
            return Ok(Outcome::failed(format!(
                "ambiguous: Claude Code session {session_id} has live session files for pids {}",
                pids.join(", ")
            )));
        }
    };

    let key = read_peer_key(sessions_dir, &session)?;
    if key.proc_start_ft != session.proc_start.to_string() || key.pid_domain != session.pid_domain {
        return Ok(Outcome::failed(format!("the peer key for pid {} does not match its session file", session.pid)));
    }

    let auth = serde_json::to_string(&AuthFrame { kind: "auth", token: &key.peer_token })?;
    let user = serde_json::to_string(&UserFrame { kind: "user", message: UserMessage { role: "user", content: frame } })?;
    Ok(pipe::write_lines(&session.messaging_socket_path, &[auth, user]).await)
}

#[cfg(test)]
#[path = "../../tests/transport/claude/mod.rs"]
mod tests;
