use anyhow::{Result, bail};

const MAX_DEPTH: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ancestor {
    pub pid: u32,
    pub start: u64,
}

/// A recorded parent pid counts only if that process started before its
/// child: the OS records the creator's pid without reparenting, so a reused
/// pid names a younger, unrelated process. The walk stops at the first parent
/// that is gone, uninspectable, or not older than its child.
pub fn build_chain(
    pid: u32,
    parent_of: impl Fn(u32) -> Option<u32>,
    start_of: impl Fn(u32) -> Option<u64>,
) -> Result<Vec<Ancestor>> {
    let Some(start) = start_of(pid) else {
        bail!("cannot read the start time of process {pid}");
    };
    let mut chain = vec![Ancestor { pid, start }];
    while chain.len() < MAX_DEPTH {
        let child = *chain.last().expect("chain is never empty");
        let Some(parent_pid) = parent_of(child.pid) else { break };
        let Some(parent_start) = start_of(parent_pid) else { break };
        if parent_start >= child.start || chain.iter().any(|a| a.pid == parent_pid) {
            break;
        }
        chain.push(Ancestor { pid: parent_pid, start: parent_start });
    }
    Ok(chain)
}

#[cfg(test)]
#[path = "../tests/process/chain/mod.rs"]
mod tests;
