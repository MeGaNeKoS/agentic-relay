use serde_json::Value;

use super::jsonrpc::{CallError, RpcError};
use crate::outcome::Outcome;

pub fn not_a_handoff(error: CallError) -> Outcome {
    Outcome::failed(error.to_string())
}

pub fn handoff(result: Result<Value, CallError>) -> Result<Value, Outcome> {
    result.map_err(|error| match error {
        CallError::Lost(reason) => Outcome::may_not_have_landed(reason),
        CallError::NotSent(reason) => Outcome::failed(reason),
        CallError::Rejected(rpc) => Outcome::failed(rpc.to_string()),
    })
}

#[derive(Debug, PartialEq, Eq)]
pub enum SteerRefusal {
    NotSteerable,
    TurnMoved,
}

pub fn classify_steer(error: &RpcError) -> Option<SteerRefusal> {
    let not_steerable = error.data.as_ref().and_then(|data| data.pointer("/codexErrorInfo/activeTurnNotSteerable")).is_some();
    let message = error.message.as_str();
    if not_steerable {
        Some(SteerRefusal::NotSteerable)
    } else if message.contains("no active turn to steer") || message.contains("expected active turn id") {
        Some(SteerRefusal::TurnMoved)
    } else {
        None
    }
}

pub fn active_turn_id(resumed: &Value) -> Option<&str> {
    let newest = resumed.pointer("/initialTurnsPage/data/0")?;
    if newest.get("status")?.as_str()? != "inProgress" {
        return None;
    }
    newest.get("id")?.as_str()
}

#[cfg(test)]
#[path = "../../tests/transport/codex/errors/mod.rs"]
mod tests;
