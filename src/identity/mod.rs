mod resolve;

pub use resolve::{CallerEnvironment, Refusal, host_environment, resolve_caller};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Identity {
    Claude { session_id: String },
    Codex { thread_id: String },
}

impl Identity {
    pub fn address(&self) -> String {
        match self {
            Self::Claude { session_id } => format!("claude {session_id}"),
            Self::Codex { thread_id } => format!("codex {thread_id}"),
        }
    }
}
