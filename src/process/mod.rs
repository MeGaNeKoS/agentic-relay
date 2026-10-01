mod chain;
#[cfg(windows)]
mod windows;

pub use chain::{Ancestor, build_chain};
#[cfg(windows)]
pub use windows::WatchedProcess;

use anyhow::Result;

pub fn start_time(pid: u32) -> Result<Option<u64>> {
    #[cfg(windows)]
    return windows::start_time(pid);
    #[cfg(not(windows))]
    anyhow::bail!("process inspection is not implemented on this platform (pid {pid})");
}

pub fn ancestry(pid: u32) -> Result<Vec<Ancestor>> {
    #[cfg(windows)]
    {
        let table = windows::process_table()?;
        build_chain(pid, |p| table.get(&p).map(|e| e.parent), |p| start_time(p).ok().flatten())
    }
    #[cfg(not(windows))]
    anyhow::bail!("process inspection is not implemented on this platform (pid {pid})");
}

pub fn describe_chain(chain: &[Ancestor]) -> String {
    #[cfg(windows)]
    {
        let table = match windows::process_table() {
            Ok(table) => table,
            Err(e) => return format!("process table unreadable: {e:#}"),
        };
        let entry = |pid: u32| table.get(&pid);
        let mut lines: Vec<String> = chain
            .iter()
            .map(|a| match entry(a.pid) {
                Some(e) => format!("pid {} parent {} image {} start {}", a.pid, e.parent, e.image, a.start),
                None => format!("pid {} (not in the process table) start {}", a.pid, a.start),
            })
            .collect();
        if let Some(top) = chain.last().and_then(|a| entry(a.pid)) {
            let above = match (entry(top.parent), start_time(top.parent)) {
                (None, _) => format!("parent {} is not running", top.parent),
                (Some(e), Ok(Some(start))) => format!("parent {} image {} start {} (not accepted)", top.parent, e.image, start),
                (Some(e), Ok(None)) => format!("parent {} image {} start time unavailable (not accepted)", top.parent, e.image),
                (Some(e), Err(err)) => format!("parent {} image {} could not be inspected: {err:#} (not accepted)", top.parent, e.image),
            };
            lines.push(format!("chain ends: {above}"));
        } else {
            lines.push("chain ends at the first step".to_string());
        }
        lines.join("; ")
    }
    #[cfg(not(windows))]
    format!("{chain:?}")
}
