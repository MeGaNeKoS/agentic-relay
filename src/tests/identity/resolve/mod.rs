mod accepted;
mod named;
mod refused;
mod thread_check;
mod unnamed;

use std::sync::Mutex;

use anyhow::{Result, bail};
use serde_json::Value;

use crate::identity::CallerEnvironment;
use crate::process::Ancestor;
use crate::protocol::Candidate;
use crate::transport::claude::SessionFile;
use crate::transport::codex::{CallError, CodexApi};

pub(super) struct FakeEnvironment {
    pub chain: Vec<Ancestor>,
    pub sessions: Vec<SessionFile>,
    pub daemon: Option<Ancestor>,
}

impl CallerEnvironment for FakeEnvironment {
    fn ancestry(&self, _pid: u32) -> Result<Vec<Ancestor>> {
        Ok(self.chain.clone())
    }

    fn claude_session(&self, pid: u32) -> Result<Option<SessionFile>> {
        Ok(self.sessions.iter().find(|s| s.pid == pid).cloned())
    }

    fn codex_daemon(&self) -> Result<Ancestor> {
        match self.daemon {
            Some(daemon) => Ok(daemon),
            None => bail!("no live daemon"),
        }
    }
}

pub(super) fn ancestor(pid: u32, start: u64) -> Ancestor {
    Ancestor { pid, start }
}

pub(super) fn session(pid: u32, session_id: &str, proc_start: u64) -> SessionFile {
    SessionFile {
        pid,
        session_id: session_id.to_string(),
        proc_start,
        pid_domain: "win32:test".to_string(),
        messaging_socket_path: String::new(),
        name: None,
    }
}

pub(super) fn claude(pid: u32, session_id: &str) -> Candidate {
    Candidate::Claude { pid, session_id: session_id.to_string() }
}

pub(super) fn codex(thread_id: &str) -> Candidate {
    Candidate::Codex { thread_id: thread_id.to_string() }
}

pub(super) struct OneRead(pub(super) Mutex<Option<Result<Value, CallError>>>);

#[async_trait::async_trait]
impl CodexApi for OneRead {
    async fn call(&self, method: &str, _params: Value) -> Result<Value, CallError> {
        assert_eq!(method, "thread/read");
        self.0.lock().unwrap().take().expect("one read only")
    }

    async fn expect_turn_started(&self, _thread_id: &str) -> Result<crate::transport::codex::TurnStartedWaiter, CallError> {
        unreachable!("identity checks never start turns")
    }
}

pub(super) fn read_of(thread: Value) -> OneRead {
    OneRead(Mutex::new(Some(Ok(thread))))
}

pub(super) struct NoCalls;

#[async_trait::async_trait]
impl CodexApi for NoCalls {
    async fn call(&self, method: &str, _params: Value) -> Result<Value, CallError> {
        panic!("unexpected call {method}");
    }

    async fn expect_turn_started(&self, _thread_id: &str) -> Result<crate::transport::codex::TurnStartedWaiter, CallError> {
        unreachable!("identity checks never start turns")
    }
}
