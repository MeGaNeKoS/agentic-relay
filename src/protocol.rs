use serde::{Deserialize, Serialize};

use crate::outcome::Outcome;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Candidate {
    Claude { pid: u32, session_id: String },
    Codex { thread_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Target {
    CodexName { name: String, cwd: String },
    CodexId { thread_id: String },
    Claude { session_id: String },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum Request {
    Send { message_id: String, caller_candidates: Vec<Candidate>, target: Target, text: String },
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Response {
    Delivered {
        message_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<String>,
    },
    Failed {
        message_id: String,
        reason: String,
    },
    MayNotHaveLanded {
        message_id: String,
        reason: String,
    },
}

impl Response {
    pub fn from_outcome(message_id: String, outcome: Outcome) -> Self {
        match outcome {
            Outcome::Delivered { note } => Self::Delivered { message_id, note },
            Outcome::Failed(reason) => Self::Failed { message_id, reason },
            Outcome::MayNotHaveLanded(reason) => Self::MayNotHaveLanded { message_id, reason },
        }
    }
}

#[cfg(test)]
#[path = "tests/protocol/mod.rs"]
mod tests;
