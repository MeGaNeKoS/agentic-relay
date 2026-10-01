mod codex_create;
mod codex_deliver;
mod codex_resolve;
mod send;
mod serve;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, bail};
use interprocess::local_socket::traits::tokio::Listener as _;

use crate::identity::{CallerEnvironment, host_environment};
use crate::lock::KeyedLocks;
use crate::transport::claude;
use crate::transport::codex::{CodexApi, CodexClient};

use codex_resolve::NameMap;

const MAX_CONSECUTIVE_ACCEPT_ERRORS: u32 = 10;
const ACCEPT_BACKOFF_START: Duration = Duration::from_millis(100);
const ACCEPT_BACKOFF_CAP: Duration = Duration::from_secs(5);

pub struct Bridge {
    codex: Arc<dyn CodexApi>,
    env: Arc<dyn CallerEnvironment>,
    sessions_dir: PathBuf,
    locks: KeyedLocks,
    names: NameMap,
}

impl Bridge {
    pub fn new(codex: Arc<dyn CodexApi>, env: Arc<dyn CallerEnvironment>, sessions_dir: PathBuf) -> Self {
        Self { codex, env, sessions_dir, locks: KeyedLocks::default(), names: NameMap::default() }
    }

    pub async fn run() -> Result<()> {
        let env = match host_environment() {
            Ok(env) => env,
            Err(refusal) => bail!("{}", refusal.0),
        };
        let bridge = Arc::new(Self::new(CodexClient::new("relay"), Arc::new(env), claude::sessions_dir()?));

        let listener = crate::local_endpoint::bind()?;
        tracing::info!("relay listening");
        let mut consecutive_errors = 0u32;
        loop {
            match listener.accept().await {
                Ok(conn) => {
                    consecutive_errors = 0;
                    let bridge = bridge.clone();
                    tokio::spawn(async move {
                        if let Err(e) = serve::handle_client(conn, bridge).await {
                            tracing::error!("client error: {e:#}");
                        }
                    });
                }
                Err(e) => {
                    consecutive_errors += 1;
                    tracing::error!("accept failed ({consecutive_errors} in a row): {e}");
                    if consecutive_errors >= MAX_CONSECUTIVE_ACCEPT_ERRORS {
                        bail!("the local endpoint failed {consecutive_errors} accepts in a row; last error: {e}");
                    }
                    tokio::time::sleep(accept_backoff(consecutive_errors)).await;
                }
            }
        }
    }
}

fn accept_backoff(consecutive_errors: u32) -> Duration {
    let doublings = consecutive_errors.saturating_sub(1).min(16);
    ACCEPT_BACKOFF_START.saturating_mul(1 << doublings).min(ACCEPT_BACKOFF_CAP)
}

#[cfg(test)]
#[path = "../tests/bridge/mod.rs"]
mod tests;

#[cfg(test)]
mod accept_backoff_tests {
    use super::*;

    #[test]
    fn backoff_doubles_and_is_capped() {
        assert_eq!(accept_backoff(1), Duration::from_millis(100));
        assert_eq!(accept_backoff(2), Duration::from_millis(200));
        assert_eq!(accept_backoff(3), Duration::from_millis(400));
        assert_eq!(accept_backoff(10), ACCEPT_BACKOFF_CAP);
        assert_eq!(accept_backoff(u32::MAX), ACCEPT_BACKOFF_CAP);
    }
}
