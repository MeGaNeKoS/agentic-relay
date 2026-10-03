use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::json;

use super::name::clean_display_name;
use super::{Caller, Identity};
use crate::process::{self, Ancestor};
use crate::protocol::Candidate;
use crate::transport::claude::{self, SessionFile};
use crate::transport::codex::{CallError, CodexApi};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal(pub String);

pub trait CallerEnvironment: Send + Sync {
    fn ancestry(&self, pid: u32) -> Result<Vec<Ancestor>>;
    fn describe_chain(&self, chain: &[Ancestor]) -> String {
        format!("{chain:?}")
    }
    fn claude_session(&self, pid: u32) -> Result<Option<SessionFile>>;
    fn codex_daemon(&self) -> Result<Ancestor>;
}

pub struct HostEnvironment {
    sessions_dir: PathBuf,
    daemon_pid_file: PathBuf,
}

impl HostEnvironment {
    pub fn from_home() -> Result<Self> {
        let home = dirs::home_dir().context("could not resolve the home directory")?;
        Ok(Self { sessions_dir: claude::sessions_dir()?, daemon_pid_file: home.join(".codex").join("app-server-daemon").join("daemon.pid") })
    }
}

#[derive(Deserialize)]
struct DaemonPidFile {
    pid: u32,
    #[serde(rename = "processStartTime")]
    process_start_time: String,
}

impl CallerEnvironment for HostEnvironment {
    fn ancestry(&self, pid: u32) -> Result<Vec<Ancestor>> {
        process::ancestry(pid)
    }

    fn describe_chain(&self, chain: &[Ancestor]) -> String {
        process::describe_chain(chain)
    }

    fn claude_session(&self, pid: u32) -> Result<Option<SessionFile>> {
        claude::read_session(&self.sessions_dir, pid)
    }

    fn codex_daemon(&self) -> Result<Ancestor> {
        let path = &self.daemon_pid_file;
        let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        let file: DaemonPidFile = serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", path.display()))?;
        let recorded: u64 = file.process_start_time.parse().with_context(|| format!("processStartTime in {} is not a number", path.display()))?;
        match process::start_time(file.pid)? {
            Some(actual) if actual == recorded => Ok(Ancestor { pid: file.pid, start: recorded }),
            _ => bail!("{} names pid {} but no running process has that start time", path.display(), file.pid),
        }
    }
}

#[derive(Debug)]
pub struct Selected {
    pub candidate: Candidate,
    pub claude_session_name: Option<String>,
}

fn distance_in_chain(candidate: &Candidate, chain: &[Ancestor], env: &dyn CallerEnvironment) -> Result<(usize, Option<String>)> {
    match candidate {
        Candidate::Claude { pid, session_id } => {
            let Some(file) = env.claude_session(*pid)? else {
                bail!("CLAUDE_PID {pid} has no session file");
            };
            if &file.session_id != session_id {
                bail!("session file for pid {pid} belongs to a different session than CLAUDE_CODE_SESSION_ID");
            }
            let distance = chain
                .iter()
                .position(|a| a.pid == *pid && a.start == file.proc_start)
                .with_context(|| format!("the Claude Code process {pid} is not an ancestor of this command"))?;
            Ok((distance, file.name))
        }
        Candidate::Codex { thread_id } => {
            let daemon = env.codex_daemon()?;
            let distance = chain
                .iter()
                .position(|a| *a == daemon)
                .with_context(|| format!("the Codex daemon is not an ancestor of this command, so CODEX_THREAD_ID {thread_id} is not trusted"))?;
            Ok((distance, None))
        }
    }
}

pub fn select_candidate(candidates: &[Candidate], chain: &[Ancestor], env: &dyn CallerEnvironment) -> Result<Selected, Refusal> {
    let mut nearest: Option<(usize, &Candidate, Option<String>)> = None;
    let mut rejections = Vec::new();
    for candidate in candidates {
        match distance_in_chain(candidate, chain, env) {
            Ok((distance, name)) if nearest.as_ref().is_none_or(|(best, ..)| distance < *best) => nearest = Some((distance, candidate, name)),
            Ok(_) => {}
            Err(e) => rejections.push(format!("{e:#}")),
        }
    }
    match nearest {
        Some((_, candidate, claude_session_name)) => Ok(Selected { candidate: candidate.clone(), claude_session_name }),
        None if candidates.is_empty() => Err(Refusal("no caller identity: the command named neither a Claude Code session nor a Codex thread".into())),
        None => Err(Refusal(format!("caller refused: {}", rejections.join("; ")))),
    }
}

pub async fn resolve_caller(
    candidates: &[Candidate],
    cli_pid: u32,
    env: &dyn CallerEnvironment,
    codex: &dyn CodexApi,
) -> Result<Caller, Refusal> {
    let chain = env.ancestry(cli_pid).map_err(|e| {
        tracing::warn!("caller refused: could not inspect the calling process {cli_pid}: {e:#}");
        Refusal(format!("could not inspect the calling process {cli_pid}: {e:#}"))
    })?;
    let Selected { candidate: chosen, claude_session_name } = select_candidate(candidates, &chain, env).inspect_err(|refusal| {
        tracing::warn!("caller refused: {}; candidates {candidates:?}; chain from pid {cli_pid}: {}", refusal.0, env.describe_chain(&chain));
    })?;
    match chosen {
        Candidate::Claude { session_id, .. } => {
            let name = claude_session_name.as_deref().and_then(clean_display_name);
            Ok(Caller { identity: Identity::Claude { session_id }, name })
        }
        Candidate::Codex { thread_id } => match codex.call("thread/read", json!({"threadId": thread_id})).await {
            Ok(read) => {
                let name = ["/thread/name", "/thread/agentNickname"].iter().find_map(|field| read.pointer(field)?.as_str().and_then(clean_display_name));
                Ok(Caller { identity: Identity::Codex { thread_id }, name })
            }
            Err(error) => {
                let reason = match error {
                    CallError::Rejected(e) => format!("caller refused: Codex thread {thread_id} was not found: {e}"),
                    e => format!("caller refused: could not verify Codex thread {thread_id}: {e}"),
                };
                tracing::warn!("{reason}; chain from pid {cli_pid}: {}", env.describe_chain(&chain));
                Err(Refusal(reason))
            }
        },
    }
}

pub fn host_environment() -> Result<HostEnvironment, Refusal> {
    HostEnvironment::from_home().map_err(|e| Refusal(format!("{e:#}")))
}

#[cfg(test)]
#[path = "../tests/identity/resolve/mod.rs"]
mod tests;
