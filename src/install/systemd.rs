use std::process::Command;

use anyhow::{bail, Context, Result};

const UNIT_NAME: &str = "relay.service";

fn unit_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::config_dir().context("could not resolve the XDG config directory")?.join("systemd").join("user"))
}

fn unit_contents(exe_path: &str) -> String {
    format!(
        "[Unit]\nDescription=relay\n\n\
         [Service]\nExecStart={exe_path} bridge\nRestart=always\nRestartSec=1\n\n\
         [Install]\nWantedBy=default.target\n"
    )
}

pub fn install() -> Result<()> {
    let exe_path = std::env::current_exe().context("current_exe")?.to_string_lossy().to_string();
    let dir = unit_dir()?;
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(UNIT_NAME), unit_contents(&exe_path))?;

    let status = Command::new("systemctl").args(["--user", "daemon-reload"]).status().context("systemctl daemon-reload")?;
    if !status.success() {
        bail!("systemctl --user daemon-reload exited {status}");
    }
    let status =
        Command::new("systemctl").args(["--user", "enable", "--now", UNIT_NAME]).status().context("systemctl enable")?;
    if !status.success() {
        bail!("systemctl --user enable --now exited {status}");
    }
    println!("installed and started {UNIT_NAME}");
    Ok(())
}

pub fn uninstall() -> Result<()> {
    let status =
        Command::new("systemctl").args(["--user", "disable", "--now", UNIT_NAME]).status().context("systemctl disable")?;
    if !status.success() {
        bail!("systemctl --user disable --now exited {status}");
    }
    match std::fs::remove_file(unit_dir()?.join(UNIT_NAME)) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e).context("removing unit file"),
    }
    let status = Command::new("systemctl").args(["--user", "daemon-reload"]).status().context("systemctl daemon-reload")?;
    if !status.success() {
        bail!("systemctl --user daemon-reload exited {status}");
    }
    println!("removed {UNIT_NAME}");
    Ok(())
}
