use anyhow::Result;
use std::io::{Read, Write};

pub trait SyncDuplex: Read + Write + Send {
    fn try_clone(&self) -> Result<Box<dyn SyncDuplex>>;
}

#[cfg(windows)]
pub fn connect(path: &str) -> Result<Box<dyn SyncDuplex>> {
    let stream = uds_windows::UnixStream::connect(path)?;
    Ok(Box::new(stream))
}

#[cfg(windows)]
impl SyncDuplex for uds_windows::UnixStream {
    fn try_clone(&self) -> Result<Box<dyn SyncDuplex>> {
        Ok(Box::new(uds_windows::UnixStream::try_clone(self)?))
    }
}

#[cfg(not(windows))]
pub fn connect(path: &str) -> Result<Box<dyn SyncDuplex>> {
    let stream = std::os::unix::net::UnixStream::connect(path)?;
    Ok(Box::new(stream))
}

#[cfg(not(windows))]
impl SyncDuplex for std::os::unix::net::UnixStream {
    fn try_clone(&self) -> Result<Box<dyn SyncDuplex>> {
        Ok(Box::new(std::os::unix::net::UnixStream::try_clone(self)?))
    }
}
