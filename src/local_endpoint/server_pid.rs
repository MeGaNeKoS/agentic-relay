use std::fs::OpenOptions;
use std::os::windows::io::AsRawHandle;

use anyhow::{Context, Result, bail};
use windows_sys::Win32::Foundation::{GetLastError, HANDLE};
use windows_sys::Win32::System::Pipes::GetNamedPipeServerProcessId;

pub fn server_pid_of(printname: &str) -> Result<Option<u32>> {
    let path = format!(r"\\.\pipe\{printname}");
    let pipe = match OpenOptions::new().read(true).write(true).open(&path) {
        Ok(pipe) => pipe,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e).with_context(|| format!("opening {path}")),
    };
    let mut pid = 0u32;
    if unsafe { GetNamedPipeServerProcessId(pipe.as_raw_handle() as HANDLE, &mut pid) } == 0 {
        bail!("GetNamedPipeServerProcessId failed with Windows error {}", unsafe { GetLastError() });
    }
    Ok(Some(pid))
}

#[cfg(test)]
#[path = "../tests/local_endpoint/server_pid/mod.rs"]
mod tests;
