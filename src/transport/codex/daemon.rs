use std::process::Stdio;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::json;
use tokio::process::Command;
use tokio::task::AbortHandle;

use super::WsConn;
use super::jsonrpc::{CALL_TIMEOUT, RpcClient};

/// `codex` is an npm shim `Command` can't exec directly; routing through
/// `cmd /C` resolves PATHEXT the way a shell would.
#[cfg(windows)]
fn codex_command() -> Command {
    let mut cmd = Command::new("cmd");
    cmd.args(["/C", "codex"]).creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    cmd
}

#[cfg(not(windows))]
fn codex_command() -> Command {
    Command::new("codex")
}

#[derive(Deserialize)]
struct DaemonVersion {
    status: String,
    #[serde(rename = "socketPath")]
    socket_path: Option<String>,
}

async fn daemon_socket_path() -> Result<String> {
    let mut cmd = codex_command();
    cmd.args(["app-server", "daemon", "version"]).stdin(Stdio::null()).kill_on_drop(true);
    let out = cmd.output().await.context("running codex app-server daemon version")?;
    if !out.status.success() {
        bail!("codex app-server daemon version exited {}: {}", out.status, String::from_utf8_lossy(&out.stderr));
    }
    let dv: DaemonVersion = serde_json::from_slice(&out.stdout).context("parsing codex app-server daemon version output")?;
    if dv.status != "running" {
        bail!("app-server daemon is not running (status {:?}); relay never starts it", dv.status);
    }
    dv.socket_path.ok_or_else(|| anyhow::anyhow!("daemon version reported no socketPath"))
}

struct AbortOnDrop(Vec<AbortHandle>);

impl AbortOnDrop {
    fn disarm(mut self) {
        self.0.clear();
    }
}

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        for handle in &self.0 {
            handle.abort();
        }
    }
}

pub async fn connect(client_name: &str) -> Result<Arc<RpcClient>> {
    let socket_path = daemon_socket_path().await?;
    let conn = WsConn::dial(socket_path).await?;
    handshake(conn, client_name, CALL_TIMEOUT).await
}

pub(super) async fn handshake(conn: WsConn, client_name: &str, call_timeout: std::time::Duration) -> Result<Arc<RpcClient>> {
    let (client, reader) = RpcClient::new(conn, call_timeout);

    let pump = tokio::spawn(client.clone().pump(reader));
    let pump_abort = pump.abort_handle();
    let supervised = client.clone();
    let supervisor = tokio::spawn(async move {
        let reason = match pump.await {
            Ok(Ok(())) => "the connection ended".to_string(),
            Ok(Err(e)) => format!("{e:#}"),
            Err(e) => format!("the connection reader stopped: {e}"),
        };
        tracing::error!("codex daemon connection lost: {reason}");
        supervised.fail_pending(&format!("connection to the codex daemon was lost: {reason}")).await;
    });
    let guard = AbortOnDrop(vec![pump_abort, supervisor.abort_handle()]);

    client
        .call(
            "initialize",
            json!({
                "clientInfo": {"name": client_name, "version": env!("CARGO_PKG_VERSION")},
                "capabilities": {"experimentalApi": true},
            }),
        )
        .await
        .context("initialize")?;
    client.notify("initialized", json!({})).await.context("initialized")?;
    guard.disarm();
    Ok(client)
}
