use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use uuid::Uuid;

use crate::local_endpoint;
use crate::protocol::{Candidate, Request, Response, Target};
use crate::transport::codex::jsonrpc::CALL_TIMEOUT;

// The longest fixed chain of daemon waits a send can make: connect (one call wait plus 30s), seven calls,
// and the 30s turn-started wait. Pagination of by-name lookups is not bounded here.
const RESPONSE_TIMEOUT: Duration = CALL_TIMEOUT.saturating_mul(8).saturating_add(Duration::from_secs(60));

const USAGE: &str = "usage: relay send claude <session id> <message>\n       relay send codex <thread id> <message>\n       relay send codex <name> <cwd> <message>";

pub fn parse_send_args(args: &[String]) -> Result<(Target, String)> {
    let parts: Vec<&str> = args.iter().map(String::as_str).collect();
    match parts.as_slice() {
        ["claude", session_id, text] => Ok((Target::Claude { session_id: (*session_id).into() }, (*text).into())),
        ["codex", thread_id, text] => Ok((Target::CodexId { thread_id: (*thread_id).into() }, (*text).into())),
        ["codex", thread_id, _, _] if Uuid::parse_str(thread_id).is_ok() => {
            bail!("`relay send codex <thread id>` takes one message argument but got extra arguments; quote the whole message as one argument\n{USAGE}")
        }
        ["codex", name, cwd, text] => Ok((Target::CodexName { name: (*name).into(), cwd: (*cwd).into() }, (*text).into())),
        _ => bail!("{USAGE}"),
    }
}

fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

pub fn collect_candidates(claude_pid: Option<String>, claude_session: Option<String>, codex_thread: Option<String>) -> Result<Vec<Candidate>> {
    let mut candidates = Vec::new();
    match (claude_pid, claude_session) {
        (Some(pid), Some(session_id)) => {
            let pid = pid.parse().context("CLAUDE_PID is not a valid process id")?;
            candidates.push(Candidate::Claude { pid, session_id });
        }
        (None, None) => {}
        _ => bail!("CLAUDE_PID and CLAUDE_CODE_SESSION_ID must be set together"),
    }
    if let Some(thread_id) = codex_thread {
        candidates.push(Candidate::Codex { thread_id });
    }
    if candidates.is_empty() {
        bail!("no caller identity: run this from inside a Claude Code session or a Codex thread");
    }
    Ok(candidates)
}

fn render(response: &Response) -> (String, ExitCode) {
    match response {
        Response::Delivered { message_id, note: None } => (format!("delivered {message_id}"), ExitCode::SUCCESS),
        Response::Delivered { message_id, note: Some(note) } => (format!("delivered {message_id}\nnote: {note}"), ExitCode::SUCCESS),
        Response::Failed { reason, .. } => (format!("failed: {reason}"), ExitCode::from(1)),
        Response::MayNotHaveLanded { message_id, reason } => (format!("may not have landed (message id {message_id}): {reason}"), ExitCode::from(2)),
    }
}

async fn exchange(request: &Request) -> Result<Option<Response>> {
    let conn = local_endpoint::connect().await?;
    if local_endpoint::write_json_line(&conn, &serde_json::to_string(request)?).await.is_err() {
        return Ok(None);
    }
    let Ok(Some(line)) = local_endpoint::read_json_line(&conn, RESPONSE_TIMEOUT).await else {
        return Ok(None);
    };
    Ok(serde_json::from_str(&line).ok())
}

pub async fn run_send(args: &[String]) -> Result<ExitCode> {
    let (target, text) = parse_send_args(args)?;
    let caller_candidates = collect_candidates(non_empty_env("CLAUDE_PID"), non_empty_env("CLAUDE_CODE_SESSION_ID"), non_empty_env("CODEX_THREAD_ID"))?;
    let message_id = Uuid::new_v4().to_string();
    let request = Request::Send { message_id: message_id.clone(), caller_candidates, target, text };

    let response = exchange(&request).await?.unwrap_or_else(|| Response::MayNotHaveLanded {
        message_id,
        reason: "the relay server's answer never arrived".to_string(),
    });
    let (line, code) = render(&response);
    if matches!(response, Response::Delivered { .. }) {
        println!("{line}");
    } else {
        eprintln!("{line}");
    }
    Ok(code)
}

#[cfg(test)]
#[path = "tests/client/mod.rs"]
mod tests;
