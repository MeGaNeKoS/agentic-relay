use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use tracing_subscriber::fmt::MakeWriter;

const MAX_BYTES: u64 = 10 * 1024 * 1024;
const KEPT_ROTATIONS: u32 = 3;

fn log_path() -> Result<PathBuf> {
    let dir = dirs::data_dir().context("could not resolve the platform's data directory for relay.log")?;
    Ok(dir.join("relay").join("relay.log"))
}

fn rotated_path(path: &Path, n: u32) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(format!(".{n}"));
    PathBuf::from(name)
}

struct RotatorState {
    path: PathBuf,
    file: File,
    size: u64,
    max_bytes: u64,
}

impl RotatorState {
    fn open(path: PathBuf, max_bytes: u64) -> Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(&path).with_context(|| format!("opening {}", path.display()))?;
        let size = file.metadata().map_or(0, |m| m.len());
        Ok(Self { path, file, size, max_bytes })
    }

    fn rotate(&mut self) -> io::Result<()> {
        for n in (1..KEPT_ROTATIONS).rev() {
            let from = rotated_path(&self.path, n);
            if from.exists() {
                fs::rename(&from, rotated_path(&self.path, n + 1))?;
            }
        }
        fs::rename(&self.path, rotated_path(&self.path, 1))?;
        self.file = OpenOptions::new().create(true).append(true).open(&self.path)?;
        self.size = 0;
        Ok(())
    }
}

#[derive(Clone)]
struct RotatingWriter(Arc<Mutex<RotatorState>>);

struct RotatingWriterGuard<'a>(std::sync::MutexGuard<'a, RotatorState>);

impl Write for RotatingWriterGuard<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.0.size + buf.len() as u64 > self.0.max_bytes {
            self.0.rotate()?;
        }
        let written = self.0.file.write(buf)?;
        self.0.size += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.file.flush()
    }
}

impl<'a> MakeWriter<'a> for RotatingWriter {
    type Writer = RotatingWriterGuard<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        RotatingWriterGuard(self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner))
    }
}

pub fn init() -> Result<()> {
    let path = log_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let state = RotatorState::open(path, MAX_BYTES)?;
    let writer = RotatingWriter(Arc::new(Mutex::new(state)));
    tracing_subscriber::fmt().with_writer(writer).with_ansi(false).init();
    Ok(())
}

#[cfg(test)]
#[path = "tests/logging/mod.rs"]
mod tests;
