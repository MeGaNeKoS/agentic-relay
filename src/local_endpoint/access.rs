use std::ptr::{null, null_mut};

use anyhow::{Context, Result, bail};
use interprocess::os::windows::security_descriptor::SecurityDescriptor;
use widestring::{U16CStr, U16CString};
use windows_sys::Win32::Foundation::{CloseHandle, ERROR_INSUFFICIENT_BUFFER, ERROR_NONE_MAPPED, GetLastError, HANDLE, LocalFree};
use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows_sys::Win32::Security::{GetTokenInformation, LookupAccountNameW, SID_NAME_USE, TOKEN_QUERY, TOKEN_USER, TokenUser};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

const SANDBOX_ACCOUNTS: [&str; 2] = ["CodexSandboxOffline", "CodexSandboxOnline"];

fn last_error() -> u32 {
    unsafe { GetLastError() }
}

fn sid_to_string(sid: *mut core::ffi::c_void) -> Result<String> {
    let mut text: *mut u16 = null_mut();
    if unsafe { ConvertSidToStringSidW(sid, &mut text) } == 0 {
        bail!("ConvertSidToStringSidW failed with Windows error {}", last_error());
    }
    // SAFETY: on success `text` is a NUL-terminated string allocated by the system.
    let string = unsafe { U16CStr::from_ptr_str(text) }.to_string_lossy();
    // SAFETY: the buffer came from ConvertSidToStringSidW, which requires LocalFree.
    unsafe { LocalFree(text.cast()) };
    Ok(string)
}

fn current_user_sid() -> Result<String> {
    let mut token: HANDLE = null_mut();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        bail!("OpenProcessToken failed with Windows error {}", last_error());
    }
    let result = token_user_sid(token);
    unsafe { CloseHandle(token) };
    result
}

fn token_user_sid(token: HANDLE) -> Result<String> {
    let mut needed = 0u32;
    unsafe { GetTokenInformation(token, TokenUser, null_mut(), 0, &mut needed) };
    if needed == 0 {
        bail!("GetTokenInformation sizing failed with Windows error {}", last_error());
    }
    let mut buffer = vec![0u64; (needed as usize).div_ceil(8)];
    // SAFETY: `buffer` is at least `needed` bytes and 8-aligned.
    if unsafe { GetTokenInformation(token, TokenUser, buffer.as_mut_ptr().cast(), needed, &mut needed) } == 0 {
        bail!("GetTokenInformation failed with Windows error {}", last_error());
    }
    // SAFETY: the buffer now holds a TOKEN_USER written by the call above.
    let user = unsafe { &*(buffer.as_ptr().cast::<TOKEN_USER>()) };
    sid_to_string(user.User.Sid)
}

fn lookup_sid(name: &str) -> Result<Option<String>> {
    let wide = U16CString::from_str(name).context("account name contains a NUL")?;
    let (mut sid_len, mut domain_len) = (0u32, 0u32);
    let mut kind: SID_NAME_USE = 0;
    unsafe { LookupAccountNameW(null(), wide.as_ptr(), null_mut(), &mut sid_len, null_mut(), &mut domain_len, &mut kind) };
    match last_error() {
        ERROR_NONE_MAPPED => return Ok(None),
        ERROR_INSUFFICIENT_BUFFER => {}
        other => bail!("LookupAccountNameW({name}) failed with Windows error {other}"),
    }
    let mut sid = vec![0u64; (sid_len as usize).div_ceil(8)];
    let mut domain = vec![0u16; domain_len as usize];
    let ok = unsafe {
        LookupAccountNameW(null(), wide.as_ptr(), sid.as_mut_ptr().cast(), &mut sid_len, domain.as_mut_ptr(), &mut domain_len, &mut kind)
    };
    if ok == 0 {
        bail!("LookupAccountNameW({name}) failed with Windows error {}", last_error());
    }
    sid_to_string(sid.as_mut_ptr().cast()).map(Some)
}

pub fn sddl(user_sid: &str, sandbox_sids: &[String]) -> String {
    let mut sddl = format!("D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;{user_sid})");
    for sid in sandbox_sids {
        sddl.push_str(&format!("(A;;GRGW;;;{sid})"));
    }
    sddl
}

/// The SIDs of the sandbox accounts that exist on this machine. The sandbox token
/// is restricted to its own user SID, so a grant to a group never passes.
fn granted_sandbox_sids(lookup: impl Fn(&str) -> Result<Option<String>>) -> Result<Vec<String>> {
    let mut granted = Vec::new();
    for account in SANDBOX_ACCOUNTS {
        match lookup(account)? {
            Some(sid) => {
                tracing::info!("endpoint grants read and write to {account} ({sid})");
                granted.push(sid);
            }
            None => tracing::warn!("no local account named {account}; the endpoint does not grant it access"),
        }
    }
    Ok(granted)
}

pub fn security_descriptor() -> Result<SecurityDescriptor> {
    let user = current_user_sid()?;
    let sandbox = granted_sandbox_sids(lookup_sid)?;
    let text = U16CString::from_str(sddl(&user, &sandbox)).context("security descriptor text contains a NUL")?;
    SecurityDescriptor::deserialize(&text).context("building the endpoint's security descriptor")
}

#[cfg(test)]
#[path = "../tests/local_endpoint/access/mod.rs"]
mod tests;
