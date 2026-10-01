use std::collections::HashMap;
use std::mem::size_of;
use std::time::Duration;

use anyhow::{Result, bail};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_INVALID_PARAMETER, FILETIME, GetLastError, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    GetExitCodeProcess, GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE, TerminateProcess, WaitForSingleObject,
};

const STILL_ACTIVE: u32 = 259;

struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

fn filetime_ticks(ft: FILETIME) -> u64 {
    (u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime)
}

pub fn start_time(pid: u32) -> Result<Option<u64>> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        let code = unsafe { GetLastError() };
        if code == ERROR_INVALID_PARAMETER {
            return Ok(None);
        }
        bail!("OpenProcess({pid}) failed with Windows error {code}");
    }
    let handle = OwnedHandle(handle);

    let mut exit_code = 0u32;
    if unsafe { GetExitCodeProcess(handle.0, &mut exit_code) } == 0 {
        bail!("GetExitCodeProcess({pid}) failed with Windows error {}", unsafe { GetLastError() });
    }
    if exit_code != STILL_ACTIVE {
        return Ok(None);
    }

    let zero = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let (mut created, mut exited, mut kernel, mut user) = (zero, zero, zero, zero);
    if unsafe { GetProcessTimes(handle.0, &mut created, &mut exited, &mut kernel, &mut user) } == 0 {
        bail!("GetProcessTimes({pid}) failed with Windows error {}", unsafe { GetLastError() });
    }
    Ok(Some(filetime_ticks(created)))
}

pub struct WatchedProcess {
    handle: OwnedHandle,
}

impl WatchedProcess {
    pub fn open(pid: u32) -> Result<Option<Self>> {
        let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE | PROCESS_TERMINATE, 0, pid) };
        if handle.is_null() {
            let code = unsafe { GetLastError() };
            if code == ERROR_INVALID_PARAMETER {
                return Ok(None);
            }
            bail!("OpenProcess({pid}) failed with Windows error {code}");
        }
        Ok(Some(Self { handle: OwnedHandle(handle) }))
    }

    pub fn terminate(&self) -> Result<()> {
        if unsafe { TerminateProcess(self.handle.0, 1) } == 0 {
            bail!("TerminateProcess failed with Windows error {}", unsafe { GetLastError() });
        }
        Ok(())
    }

    pub fn wait(&self, timeout: Duration) -> Result<bool> {
        let millis = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX - 1);
        match unsafe { WaitForSingleObject(self.handle.0, millis) } {
            WAIT_OBJECT_0 => Ok(true),
            WAIT_TIMEOUT => Ok(false),
            _ => bail!("WaitForSingleObject failed with Windows error {}", unsafe { GetLastError() }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessEntry {
    pub parent: u32,
    pub image: String,
}

pub fn process_table() -> Result<HashMap<u32, ProcessEntry>> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        bail!("CreateToolhelp32Snapshot failed with Windows error {}", unsafe { GetLastError() });
    }
    let snapshot = OwnedHandle(snapshot);

    // SAFETY: PROCESSENTRY32W is plain data, so all-zero is valid before dwSize is set.
    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
    let mut parents = HashMap::new();
    let mut more = unsafe { Process32FirstW(snapshot.0, &mut entry) } != 0;
    while more {
        let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
        let image = String::from_utf16_lossy(&entry.szExeFile[..len]);
        parents.insert(entry.th32ProcessID, ProcessEntry { parent: entry.th32ParentProcessID, image });
        more = unsafe { Process32NextW(snapshot.0, &mut entry) } != 0;
    }
    Ok(parents)
}

#[cfg(test)]
#[path = "../tests/process/windows/mod.rs"]
mod tests;
