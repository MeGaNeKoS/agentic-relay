use serde_json::{Value, json};

use crate::outcome::Outcome;
use crate::transport::codex::errors::{SteerRefusal, active_turn_id, classify_steer, handoff, not_a_handoff};
use crate::transport::codex::{CallError, CodexApi};

pub fn user_input(text: &str) -> Value {
    json!([{"type": "text", "text": text, "text_elements": []}])
}

enum Step {
    Done(Outcome),
    SteerRefused { turn_id: Option<String> },
}

fn resume_params(thread_id: &str) -> Value {
    json!({"threadId": thread_id, "excludeTurns": true, "initialTurnsPage": {"limit": 1}})
}

pub async fn deliver(codex: &dyn CodexApi, thread_id: &str, message_id: &str, frame: &str) -> Outcome {
    let outcome = deliver_inner(codex, thread_id, message_id, frame).await;
    if let Err(e) = codex.call("thread/unsubscribe", json!({"threadId": thread_id})).await {
        tracing::warn!("thread/unsubscribe {thread_id}: {e}");
    }
    outcome
}

async fn deliver_inner(codex: &dyn CodexApi, thread_id: &str, message_id: &str, frame: &str) -> Outcome {
    let first = match resume(codex, thread_id).await {
        Ok(resumed) => resumed,
        Err(outcome) => return outcome,
    };
    let refused_turn = match act(codex, thread_id, &first, message_id, frame).await {
        Step::Done(outcome) => return outcome,
        Step::SteerRefused { turn_id } => turn_id,
    };

    let second = match resume(codex, thread_id).await {
        Ok(resumed) => resumed,
        Err(outcome) => return outcome,
    };
    let still_running = refused_turn.is_some() && active_turn_id(&second) == refused_turn.as_deref();
    if still_running {
        return queue(codex, thread_id, message_id, frame).await;
    }
    match act(codex, thread_id, &second, message_id, frame).await {
        Step::Done(outcome) => outcome,
        Step::SteerRefused { .. } => Outcome::failed("target busy, retry"),
    }
}

async fn resume(codex: &dyn CodexApi, thread_id: &str) -> Result<Value, Outcome> {
    codex.call("thread/resume", resume_params(thread_id)).await.map_err(not_a_handoff)
}

async fn act(codex: &dyn CodexApi, thread_id: &str, resumed: &Value, message_id: &str, frame: &str) -> Step {
    let status = resumed.pointer("/thread/status/type").and_then(Value::as_str);
    match status {
        Some("idle") => Step::Done(start_turn(codex, thread_id, message_id, frame).await),
        Some("active") => match active_turn_id(resumed) {
            Some(turn_id) => steer(codex, thread_id, turn_id, message_id, frame).await,
            None => Step::SteerRefused { turn_id: None },
        },
        Some("systemError") => Step::Done(Outcome::failed(format!("thread {thread_id} is in a system error state"))),
        Some(other) => Step::Done(Outcome::failed(format!("thread {thread_id} reported the unexpected status {other:?}"))),
        None => Step::Done(Outcome::failed(format!("thread/resume for {thread_id} reported no status"))),
    }
}

fn delivered_or(result: Result<Value, CallError>) -> Outcome {
    match handoff(result) {
        Ok(_) => Outcome::delivered(),
        Err(outcome) => outcome,
    }
}

async fn start_turn(codex: &dyn CodexApi, thread_id: &str, message_id: &str, frame: &str) -> Outcome {
    let params = json!({"threadId": thread_id, "clientUserMessageId": message_id, "input": user_input(frame)});
    delivered_or(codex.call("turn/start", params).await)
}

async fn steer(codex: &dyn CodexApi, thread_id: &str, turn_id: &str, message_id: &str, frame: &str) -> Step {
    let params = json!({"threadId": thread_id, "expectedTurnId": turn_id, "clientUserMessageId": message_id, "input": user_input(frame)});
    match codex.call("turn/steer", params).await {
        Err(CallError::Rejected(e)) => match classify_steer(&e) {
            Some(SteerRefusal::NotSteerable) => Step::Done(queue(codex, thread_id, message_id, frame).await),
            Some(SteerRefusal::TurnMoved) => Step::SteerRefused { turn_id: Some(turn_id.to_string()) },
            None => Step::Done(Outcome::failed(e.to_string())),
        },
        other => Step::Done(delivered_or(other)),
    }
}

async fn queue(codex: &dyn CodexApi, thread_id: &str, message_id: &str, frame: &str) -> Outcome {
    let params = json!({"threadId": thread_id, "clientUserMessageId": message_id, "input": user_input(frame)});
    delivered_or(codex.call("thread/queue/add", params).await)
}
