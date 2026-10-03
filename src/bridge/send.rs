use std::path::Path;
use std::time::Duration;

use uuid::Uuid;

use super::{Bridge, codex_create, codex_deliver, codex_resolve};
use crate::frame;
use crate::identity::{Caller, Identity, Refusal, resolve_caller};
use crate::outcome::Outcome;
use crate::protocol::{Candidate, Target};
use crate::transport::claude;
use crate::transport::norm_cwd;

const SELF_SEND: &str = "refusing to send to yourself";
const TURN_STARTED_TIMEOUT: Duration = Duration::from_secs(30);

impl Bridge {
    pub async fn send(&self, cli_pid: u32, message_id: &str, candidates: &[Candidate], target: &Target, text: &str) -> Outcome {
        let mut caller = None;
        let outcome = self.send_as(&mut caller, cli_pid, message_id, candidates, target, text).await;
        let caller = caller.map_or_else(|| "unidentified".to_string(), |c| c.address());
        tracing::info!("send {message_id} from {caller} (pid {cli_pid}) to {target:?}: {outcome:?}");
        outcome
    }

    async fn send_as(
        &self,
        caller: &mut Option<Identity>,
        cli_pid: u32,
        message_id: &str,
        candidates: &[Candidate],
        target: &Target,
        text: &str,
    ) -> Outcome {
        if Uuid::parse_str(message_id).is_err() {
            return Outcome::failed("message_id must be a UUID");
        }
        if text.is_empty() {
            return Outcome::failed("the message is empty");
        }
        let Caller { identity: sender, name } = match resolve_caller(candidates, cli_pid, &*self.env, &*self.codex).await {
            Ok(resolved) => resolved,
            Err(Refusal(reason)) => return Outcome::failed(reason),
        };
        *caller = Some(sender.clone());
        let frame = frame::render(message_id, &sender, name.as_deref(), text);

        match target {
            Target::Claude { session_id } => {
                if sender == (Identity::Claude { session_id: session_id.clone() }) {
                    return Outcome::failed(SELF_SEND);
                }
                let _target = self.locks.acquire(format!("claude:{session_id}")).await;
                claude::deliver(&self.sessions_dir, session_id, &frame).await
            }
            Target::CodexId { thread_id } => self.send_to_thread(&sender, thread_id, message_id, &frame).await,
            Target::CodexName { name, cwd } => self.send_by_name(&sender, name, cwd, message_id, &frame).await,
        }
    }

    async fn send_to_thread(&self, sender: &Identity, thread_id: &str, message_id: &str, frame: &str) -> Outcome {
        if *sender == (Identity::Codex { thread_id: thread_id.to_string() }) {
            return Outcome::failed(SELF_SEND);
        }
        let _target = self.locks.acquire(format!("codex:{thread_id}")).await;
        codex_deliver::deliver(&*self.codex, thread_id, message_id, frame).await
    }

    async fn send_by_name(&self, sender: &Identity, name: &str, cwd: &str, message_id: &str, frame: &str) -> Outcome {
        if !Path::new(cwd).is_absolute() {
            return Outcome::failed(format!("the cwd must be an absolute path, got {cwd:?}"));
        }
        let _name = self.locks.acquire(format!("codex-name:{name}\0{}", norm_cwd(cwd))).await;
        match codex_resolve::lookup(&*self.codex, &self.names, name, cwd).await {
            Err(outcome) => outcome,
            Ok(codex_resolve::Lookup::Found(thread_id)) => self.send_to_thread(sender, &thread_id, message_id, frame).await,
            Ok(codex_resolve::Lookup::Missing) => codex_create::create_and_deliver(&*self.codex, &self.names, TURN_STARTED_TIMEOUT, name, cwd, message_id, frame).await,
        }
    }
}
