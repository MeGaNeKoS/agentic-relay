use std::process::Command;
use std::time::Duration;

use anyhow::{bail, Context, Result};

use crate::local_endpoint;
use crate::process::WatchedProcess;

const TASK_NAME: &str = "Relay";

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

fn task_xml(exe_path: &str, author: &str, user_id: &str) -> String {
    let exe_path = xml_escape(exe_path);
    let author = xml_escape(author);
    let user_id = xml_escape(user_id);
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.4" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Author>{author}</Author>
    <Description>Always-on relay process, started at logon.</Description>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <UserId>{user_id}</UserId>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{user_id}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>true</Hidden>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>7</Priority>
    <RestartOnFailure>
      <Interval>PT1M</Interval>
      <Count>999</Count>
    </RestartOnFailure>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{exe_path}</Command>
      <Arguments>bridge</Arguments>
    </Exec>
  </Actions>
</Task>"#
    )
}

fn current_user_id() -> Result<String> {
    let out = Command::new("whoami")
        .output()
        .context("running whoami")?;
    if !out.status.success() {
        bail!("whoami exited {}", out.status);
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn is_access_denied(stderr: &str) -> bool {
    stderr.to_lowercase().contains("access is denied")
}

pub fn install() -> Result<()> {
    let exe_path = std::env::current_exe().context("current_exe")?;
    let exe_path = exe_path.to_string_lossy().to_string();
    let user_id = current_user_id()?;

    let xml = task_xml(&exe_path, &user_id, &user_id);
    let xml_path = std::env::temp_dir().join("relay-task.xml");
    // schtasks reads the XML as UTF-16LE when the declaration says so.
    let utf16: Vec<u8> = xml.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mut bom_and_body = vec![0xFFu8, 0xFE];
    bom_and_body.extend(utf16);
    std::fs::write(&xml_path, bom_and_body).context("writing task XML")?;

    let out = Command::new("schtasks")
        .args(["/Create", "/TN", TASK_NAME, "/XML"])
        .arg(&xml_path)
        .arg("/F")
        .output()
        .context("running schtasks /Create")?;
    let _ = std::fs::remove_file(&xml_path);
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        if is_access_denied(&stderr) {
            bail!(
                "registering the logon task needs an elevated (administrator) shell; \
                 run `relay install` again from one. The bridge process itself \
                 still runs as {user_id} at least privilege once the task is registered."
            );
        }
        bail!("schtasks /Create exited {}: {stderr}", out.status);
    }

    let run_out = Command::new("schtasks").args(["/Run", "/TN", TASK_NAME]).output().context("running schtasks /Run")?;
    if !run_out.status.success() {
        bail!("schtasks /Run exited {}: {}", run_out.status, String::from_utf8_lossy(&run_out.stderr));
    }

    println!("installed and started logon task {TASK_NAME:?} running {exe_path} bridge as {user_id}");
    Ok(())
}

#[derive(Debug)]
pub struct ElevationRequired;

impl std::fmt::Display for ElevationRequired {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("removing the logon task needs an elevated (administrator) shell; run `relay uninstall` again from one.")
    }
}

impl std::error::Error for ElevationRequired {}

const DELETE_TASK_ARGS: [&str; 4] = ["/Delete", "/TN", TASK_NAME, "/F"];
const SERVER_EXIT_WAIT: Duration = Duration::from_secs(10);

fn delete_failure(status: &str, stderr: &str) -> anyhow::Error {
    if is_access_denied(stderr) {
        return ElevationRequired.into();
    }
    anyhow::anyhow!("schtasks {} exited {status}: {stderr}", DELETE_TASK_ARGS[0])
}

fn delete_task() -> Result<()> {
    let out = Command::new("schtasks").args(DELETE_TASK_ARGS).output().context("running schtasks /Delete")?;
    if !out.status.success() {
        return Err(delete_failure(&out.status.to_string(), &String::from_utf8_lossy(&out.stderr)));
    }
    Ok(())
}

fn stop_server(pid: u32, watch: &WatchedProcess) -> Result<()> {
    watch.terminate().with_context(|| format!("ending the relay server (pid {pid})"))?;
    if !watch.wait(SERVER_EXIT_WAIT)? {
        bail!("the relay server (pid {pid}) was still running {SERVER_EXIT_WAIT:?} after it was ended; end it with `taskkill /F /PID {pid}`");
    }
    Ok(())
}

pub fn uninstall() -> Result<()> {
    let running = match local_endpoint::running_server_pid()? {
        Some(pid) => WatchedProcess::open(pid)?.map(|watch| (pid, watch)),
        None => None,
    };
    delete_task()?;
    if let Some((pid, watch)) = running {
        stop_server(pid, &watch)?;
    }
    println!("stopped and removed logon task {TASK_NAME:?}");
    Ok(())
}

#[cfg(test)]
#[path = "../tests/install/windows/mod.rs"]
mod tests;
