#[cfg(windows)]
mod access;
#[cfg(windows)]
mod server_pid;

use std::time::Duration;

use anyhow::{Context, Result, bail};
use interprocess::local_socket::{
    GenericFilePath, GenericNamespaced, ListenerOptions,
    tokio::{Listener, Stream, prelude::*},
    traits::StreamCommon,
};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

const MAX_LINE_BYTES: u64 = 8 * 1024 * 1024;
pub const REQUEST_READ_TIMEOUT: Duration = Duration::from_secs(10);

fn current_username() -> Result<String> {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .context("could not resolve the current username for the local endpoint name")
}

fn endpoint_printname() -> Result<String> {
    Ok(format!("relay-{}.sock", current_username()?))
}

fn endpoint_name() -> Result<interprocess::local_socket::Name<'static>> {
    endpoint_name_for(&endpoint_printname()?)
}

pub(crate) fn endpoint_name_for(printname: &str) -> Result<interprocess::local_socket::Name<'static>> {
    if GenericNamespaced::is_supported() {
        Ok(printname.to_ns_name::<GenericNamespaced>()?.into_owned())
    } else {
        let dir = dirs::runtime_dir().context("could not resolve the platform's runtime directory for the local endpoint socket")?;
        Ok(dir.join(printname).to_fs_name::<GenericFilePath>()?.into_owned())
    }
}

#[cfg(windows)]
pub fn running_server_pid() -> Result<Option<u32>> {
    server_pid::server_pid_of(&endpoint_printname()?)
}

pub fn bind() -> Result<Listener> {
    bind_name(endpoint_name()?)
}

/// A named pipe whose first instance already exists answers a second creator with
/// access denied rather than address in use.
fn is_endpoint_taken(e: &std::io::Error) -> bool {
    e.kind() == std::io::ErrorKind::AddrInUse || (cfg!(windows) && e.kind() == std::io::ErrorKind::PermissionDenied)
}

pub(crate) fn bind_name(name: interprocess::local_socket::Name<'static>) -> Result<Listener> {
    let options = ListenerOptions::new().name(name);
    #[cfg(windows)]
    let options = {
        use interprocess::os::windows::local_socket::ListenerOptionsExt;
        options.security_descriptor(access::security_descriptor()?)
    };
    match options.create_tokio() {
        Ok(l) => Ok(l),
        Err(e) if is_endpoint_taken(&e) => {
            bail!("relay is already running (endpoint in use); refusing to bind a second server")
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn connect() -> Result<Stream> {
    let name = endpoint_name()?;
    Stream::connect(name)
        .await
        .context("relay is not running; start it with the installed logon task, or run `relay bridge` yourself")
}

pub fn peer_pid(stream: &Stream) -> Result<u32> {
    let creds = stream.peer_creds().context("reading the peer credentials of the connection")?;
    creds.pid().context("this platform's local socket does not report the peer's process id")
}

pub async fn read_json_line(stream: &Stream, timeout: Duration) -> Result<Option<String>> {
    let mut reader = BufReader::new(stream.take(MAX_LINE_BYTES));
    let mut line = String::new();
    let read = tokio::time::timeout(timeout, reader.read_line(&mut line)).await.context("timed out waiting for a line")??;
    if read == 0 || !line.ends_with('\n') {
        return Ok(None);
    }
    Ok(Some(line))
}

pub async fn write_json_line(stream: &Stream, line: &str) -> Result<()> {
    let mut buffer = String::with_capacity(line.len() + 1);
    buffer.push_str(line);
    buffer.push('\n');
    let mut w = stream;
    w.write_all(buffer.as_bytes()).await?;
    w.flush().await?;
    Ok(())
}

#[cfg(test)]
#[path = "../tests/local_endpoint/mod.rs"]
mod tests;
