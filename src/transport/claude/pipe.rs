use std::time::Duration;

use tokio::io::AsyncWriteExt;

use crate::outcome::Outcome;

const WRITE_TIMEOUT: Duration = Duration::from_secs(10);

#[cfg(windows)]
const ERROR_PIPE_BUSY: i32 = 231;
#[cfg(windows)]
const BUSY_RETRIES: u32 = 5;
#[cfg(windows)]
const BUSY_RETRY_DELAY: Duration = Duration::from_millis(100);

#[cfg(windows)]
type Client = tokio::net::windows::named_pipe::NamedPipeClient;
#[cfg(not(windows))]
type Client = tokio::net::UnixStream;

#[cfg(windows)]
async fn open(path: &str) -> std::io::Result<Client> {
    use tokio::net::windows::named_pipe::ClientOptions;

    let mut attempts = 0;
    loop {
        match ClientOptions::new().open(path) {
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) && attempts < BUSY_RETRIES => {
                attempts += 1;
                tokio::time::sleep(BUSY_RETRY_DELAY).await;
            }
            other => return other,
        }
    }
}

#[cfg(not(windows))]
async fn open(path: &str) -> std::io::Result<Client> {
    tokio::net::UnixStream::connect(path).await
}

pub async fn write_lines(path: &str, lines: &[String]) -> Outcome {
    let mut client = match tokio::time::timeout(WRITE_TIMEOUT, open(path)).await {
        Ok(Ok(client)) => client,
        Ok(Err(e)) => return Outcome::failed(format!("opening the session's inbox pipe {path}: {e}")),
        Err(_) => return Outcome::failed(format!("opening the session's inbox pipe {path} took longer than {WRITE_TIMEOUT:?}")),
    };
    let write = async {
        for line in lines {
            client.write_all(line.as_bytes()).await?;
            client.write_all(b"\n").await?;
        }
        client.flush().await
    };
    match tokio::time::timeout(WRITE_TIMEOUT, write).await {
        Ok(Ok(())) => Outcome::delivered(),
        Ok(Err(e)) => Outcome::may_not_have_landed(format!("writing to the session's inbox pipe failed: {e}")),
        Err(_) => Outcome::may_not_have_landed(format!("writing to the session's inbox pipe took longer than {WRITE_TIMEOUT:?}")),
    }
}
