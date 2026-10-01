use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct SessionFile {
    pub pid: u32,
    pub session_id: String,
    pub proc_start: u64,
    pub pid_domain: String,
    pub messaging_socket_path: String,
}

#[derive(Deserialize)]
struct RawSession {
    pid: u32,
    #[serde(rename = "sessionId")]
    session_id: String,
    #[serde(rename = "procStart")]
    proc_start: String,
    #[serde(rename = "pidDomain")]
    pid_domain: String,
    #[serde(rename = "messagingSocketPath")]
    messaging_socket_path: String,
}

pub fn sessions_dir() -> Result<PathBuf> {
    Ok(dirs::home_dir().context("could not resolve the home directory for ~/.claude/sessions")?.join(".claude").join("sessions"))
}

fn parse(path: &Path, bytes: &[u8]) -> Result<SessionFile> {
    let raw: RawSession = serde_json::from_slice(bytes).with_context(|| format!("parsing {}", path.display()))?;
    let proc_start = raw.proc_start.parse().with_context(|| format!("procStart in {} is not a number", path.display()))?;
    Ok(SessionFile {
        pid: raw.pid,
        session_id: raw.session_id,
        proc_start,
        pid_domain: raw.pid_domain,
        messaging_socket_path: raw.messaging_socket_path,
    })
}

pub fn read_session(dir: &Path, pid: u32) -> Result<Option<SessionFile>> {
    let path = dir.join(format!("{pid}.json"));
    match std::fs::read(&path) {
        Ok(bytes) => parse(&path, &bytes).map(Some),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
    }
}

pub fn list_sessions(dir: &Path) -> Result<Vec<SessionFile>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e).with_context(|| format!("listing {}", dir.display())),
    };
    let mut sessions = Vec::new();
    for entry in entries {
        let path = entry.with_context(|| format!("listing {}", dir.display()))?.path();
        let is_pid_file = path.extension().is_some_and(|e| e == "json")
            && path.file_stem().and_then(|s| s.to_str()).is_some_and(|s| s.parse::<u32>().is_ok());
        if !is_pid_file {
            continue;
        }
        let parsed = std::fs::read(&path).with_context(|| format!("reading {}", path.display())).and_then(|b| parse(&path, &b));
        match parsed {
            Ok(session) => sessions.push(session),
            Err(e) => tracing::warn!("skipping session file: {e:#}"),
        }
    }
    Ok(sessions)
}
