use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::{Value, json};

use crate::outcome::Outcome;
use crate::transport::codex::errors::not_a_handoff;
use crate::transport::codex::{CallError, CodexApi};
use crate::transport::norm_cwd;

const MAX_LIST_PAGES: usize = 200;

#[derive(Debug, PartialEq, Eq)]
pub enum Lookup {
    Found(String),
    Missing,
}

#[derive(Default)]
pub struct NameMap(Mutex<HashMap<(String, String), String>>);

impl NameMap {
    fn key(name: &str, cwd: &str) -> (String, String) {
        (name.to_string(), norm_cwd(cwd))
    }

    fn entries(&self) -> std::sync::MutexGuard<'_, HashMap<(String, String), String>> {
        self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn insert(&self, name: &str, cwd: &str, thread_id: &str) {
        self.entries().insert(Self::key(name, cwd), thread_id.to_string());
    }

    fn get(&self, name: &str, cwd: &str) -> Option<String> {
        self.entries().get(&Self::key(name, cwd)).cloned()
    }

    fn remove(&self, name: &str, cwd: &str) {
        self.entries().remove(&Self::key(name, cwd));
    }
}

fn string_field<'a>(thread: &'a Value, key: &str) -> Result<&'a str, Outcome> {
    thread.get(key).and_then(Value::as_str).ok_or_else(|| Outcome::failed(format!("thread/list entry has no string {key:?}: {thread}")))
}

async fn list_matches(codex: &dyn CodexApi, name: &str, cwd: &str) -> Result<Vec<String>, Outcome> {
    let wanted_cwd = norm_cwd(cwd);
    let mut ids = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..MAX_LIST_PAGES {
        let mut params = json!({"searchTerm": name});
        if let Some(cursor) = &cursor {
            params["cursor"] = json!(cursor);
        }
        let page = codex.call("thread/list", params).await.map_err(not_a_handoff)?;
        let data = page.get("data").and_then(Value::as_array).ok_or_else(|| Outcome::failed("thread/list answered with no data array"))?;
        for thread in data {
            if thread.get("name").and_then(Value::as_str) != Some(name) {
                continue;
            }
            if norm_cwd(string_field(thread, "cwd")?) == wanted_cwd {
                ids.push(string_field(thread, "id")?.to_string());
            }
        }
        cursor = page.get("nextCursor").and_then(Value::as_str).map(str::to_string);
        if cursor.is_none() {
            return Ok(ids);
        }
    }
    Err(Outcome::failed(format!("thread/list kept returning pages after {MAX_LIST_PAGES}")))
}

async fn mapped_thread(codex: &dyn CodexApi, names: &NameMap, name: &str, cwd: &str) -> Result<Option<String>, Outcome> {
    let Some(thread_id) = names.get(name, cwd) else { return Ok(None) };
    match codex.call("thread/read", json!({"threadId": thread_id})).await {
        Ok(read) => {
            let thread = read.get("thread");
            let same_name = thread.and_then(|t| t.get("name")).and_then(Value::as_str) == Some(name);
            let same_cwd = thread.and_then(|t| t.get("cwd")).and_then(Value::as_str).is_some_and(|c| norm_cwd(c) == norm_cwd(cwd));
            if same_name && same_cwd {
                return Ok(Some(thread_id));
            }
        }
        Err(CallError::Rejected(_)) => {}
        Err(e) => return Err(not_a_handoff(e)),
    }
    names.remove(name, cwd);
    Ok(None)
}

pub async fn lookup(codex: &dyn CodexApi, names: &NameMap, name: &str, cwd: &str) -> Result<Lookup, Outcome> {
    let mut ids = list_matches(codex, name, cwd).await?;
    if let Some(mapped) = mapped_thread(codex, names, name, cwd).await? {
        ids.push(mapped);
    }
    ids.sort();
    ids.dedup();
    match ids.len() {
        0 => Ok(Lookup::Missing),
        1 => Ok(Lookup::Found(ids.remove(0))),
        _ => Err(Outcome::failed(format!("ambiguous: {} threads are named {name:?} in {cwd}: {}", ids.len(), ids.join(", ")))),
    }
}
