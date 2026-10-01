use std::time::Duration;

use serde_json::{Value, json};

use super::codex_deliver::user_input;
use super::codex_resolve::NameMap;
use crate::outcome::Outcome;
use crate::transport::codex::{CallError, CodexApi};

/// Starts a thread, delivers into it, then names it once the daemon announces the
/// turn: before that the rollout can still be empty and naming fails. Naming comes last: a named
/// thread with no turn stays on disk unlisted, where a later by-name send can't
/// find it, while an unnamed thread with no turn leaves nothing behind.
pub async fn create_and_deliver(
    codex: &dyn CodexApi,
    names: &NameMap,
    naming_wait: Duration,
    name: &str,
    cwd: &str,
    message_id: &str,
    frame: &str,
) -> Outcome {
    let started = match codex.call("thread/start", json!({"cwd": cwd})).await {
        Ok(started) => started,
        Err(CallError::Lost(reason)) => {
            return Outcome::may_not_have_landed(format!(
                "thread/start got no answer ({reason}); an unnamed thread may exist and nothing was sent to it"
            ));
        }
        Err(e) => return Outcome::failed(e.to_string()),
    };
    let Some(thread_id) = started.pointer("/thread/id").and_then(Value::as_str) else {
        return Outcome::failed("thread/start answered with no thread id");
    };

    let outcome = deliver_then_name(codex, names, naming_wait, thread_id, name, cwd, (message_id, frame)).await;
    if let Err(e) = codex.call("thread/unsubscribe", json!({"threadId": thread_id})).await {
        tracing::warn!("thread/unsubscribe {thread_id}: {e}");
    }
    outcome
}

async fn deliver_then_name(
    codex: &dyn CodexApi,
    names: &NameMap,
    naming_wait: Duration,
    thread_id: &str,
    name: &str,
    cwd: &str,
    message: (&str, &str),
) -> Outcome {
    let (message_id, frame) = message;
    let started = match codex.expect_turn_started(thread_id).await {
        Ok(waiter) => waiter,
        Err(e) => return Outcome::failed(e.to_string()),
    };
    let turn = json!({"threadId": thread_id, "clientUserMessageId": message_id, "input": user_input(frame)});
    match codex.call("turn/start", turn).await {
        Ok(_) => {}
        Err(CallError::Lost(reason)) => {
            return Outcome::may_not_have_landed(format!(
                "turn/start got no answer ({reason}); thread {thread_id} is unnamed and may hold the message"
            ));
        }
        Err(e) => return Outcome::failed(e.to_string()),
    }
    if !started.wait(naming_wait).await {
        return Outcome::Delivered {
            note: Some(format!("thread {thread_id} is unnamed: no turn/started announcement arrived within {naming_wait:?}")),
        };
    }
    match codex.call("thread/name/set", json!({"threadId": thread_id, "name": name})).await {
        Ok(_) => {
            names.insert(name, cwd, thread_id);
            Outcome::delivered()
        }
        Err(e) => Outcome::Delivered { note: Some(format!("thread {thread_id} is unnamed: thread/name/set failed: {e}")) },
    }
}
