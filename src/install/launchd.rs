use std::process::Command;

use anyhow::{bail, Context, Result};

const LABEL: &str = "com.relay.agent";

fn plist_path() -> Result<std::path::PathBuf> {
    Ok(dirs::home_dir().context("could not resolve the home directory")?.join("Library").join("LaunchAgents").join(format!("{LABEL}.plist")))
}

fn plist_contents(exe_path: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe_path}</string>
        <string>bridge</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
</dict>
</plist>
"#
    )
}

pub fn install() -> Result<()> {
    let exe_path = std::env::current_exe().context("current_exe")?.to_string_lossy().to_string();
    let path = plist_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, plist_contents(&exe_path))?;

    let status = Command::new("launchctl").args(["load", "-w"]).arg(&path).status().context("launchctl load")?;
    if !status.success() {
        bail!("launchctl load -w exited {status}");
    }
    println!("installed and loaded {LABEL}");
    Ok(())
}

pub fn uninstall() -> Result<()> {
    let path = plist_path()?;
    let status = Command::new("launchctl").args(["unload", "-w"]).arg(&path).status().context("launchctl unload")?;
    if !status.success() {
        bail!("launchctl unload -w exited {status}");
    }
    match std::fs::remove_file(&path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e).context("removing plist"),
    }
    println!("removed {LABEL}");
    Ok(())
}
