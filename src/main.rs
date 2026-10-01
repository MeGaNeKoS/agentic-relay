mod bridge;
mod client;
mod frame;
mod identity;
mod install;
mod local_endpoint;
mod lock;
mod logging;
mod outcome;
mod process;
mod protocol;
mod transport;

use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "relay")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Runs the always-on relay server. Installed to start at logon by
    /// `install`; can also be run directly.
    Bridge,
    /// Registers per-user startup at logon and starts the server now.
    Install,
    /// Removes what `install` registered.
    Uninstall,
    /// Sends a text message to a Claude Code session or a Codex thread.
    ///
    /// relay send claude <session id> <message>
    /// relay send codex <thread id> <message>
    /// relay send codex <name> <cwd> <message>
    Send {
        #[arg(required = true, num_args = 3..=4)]
        args: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> Result<ExitCode> {
    match Cli::parse().command {
        Command::Bridge => {
            detach_console();
            logging::init()?;
            if let Err(e) = bridge::Bridge::run().await {
                tracing::error!("relay server exited: {e:#}");
                return Err(e);
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Install => install::install().map(|()| ExitCode::SUCCESS),
        Command::Uninstall => install::uninstall().map(|()| ExitCode::SUCCESS),
        Command::Send { args } => client::run_send(&args).await,
    }
}

/// The binary is a console program so `relay send` is waited on and prints like
/// any command; the server detaches from the console the logon task gave it.
fn detach_console() {
    #[cfg(windows)]
    unsafe {
        windows_sys::Win32::System::Console::FreeConsole();
    }
}
