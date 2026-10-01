use std::ffi::OsString;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

const SKILL_FILE: &str = "SKILL.md";
const EXE_PLACEHOLDER: &str = "@RELAY@";
const CODEX_RESTART_NOTE: &str = "A running Codex daemon loads skills only when it starts; run `codex app-server daemon restart` to make it pick up the new skill.";

pub struct Agent {
    label: &'static str,
    config_env: &'static str,
    default_folder: &'static str,
    skill: &'static str,
    written_note: Option<&'static str>,
}

const CLAUDE: Agent = Agent {
    label: "Claude Code",
    config_env: "CLAUDE_CONFIG_DIR",
    default_folder: ".claude",
    skill: include_str!("../../skills/claude/relay/SKILL.md"),
    written_note: None,
};

const CODEX: Agent = Agent {
    label: "Codex",
    config_env: "CODEX_HOME",
    default_folder: ".codex",
    skill: include_str!("../../skills/codex/relay/SKILL.md"),
    written_note: Some(CODEX_RESTART_NOTE),
};

const AGENTS: [Agent; 2] = [CLAUDE, CODEX];

#[derive(Debug, PartialEq, Eq)]
enum Change {
    Written(PathBuf),
    Skipped(PathBuf),
    Removed(PathBuf),
    Absent(PathBuf),
}

fn config_dir(agent: &Agent, env_value: Option<OsString>, home: &Path) -> PathBuf {
    match env_value.filter(|value| !value.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => home.join(agent.default_folder),
    }
}

fn skill_dir(config: &Path) -> PathBuf {
    config.join("skills").join("relay")
}

fn command_path(exe: &Path) -> String {
    let path = exe.to_string_lossy().replace('\\', "/");
    if path.contains(char::is_whitespace) { format!("\"{path}\"") } else { path }
}

fn render(agent: &Agent, exe: &Path) -> String {
    agent.skill.replace(EXE_PLACEHOLDER, &command_path(exe))
}

fn write_skill(agent: &Agent, config: &Path, exe: &Path) -> Result<Change> {
    if !config.is_dir() {
        return Ok(Change::Skipped(config.to_path_buf()));
    }
    let dir = skill_dir(config);
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let file = dir.join(SKILL_FILE);
    std::fs::write(&file, render(agent, exe)).with_context(|| format!("writing {}", file.display()))?;
    Ok(Change::Written(file))
}

fn remove_skill(config: &Path) -> Result<Change> {
    let dir = skill_dir(config);
    let file = dir.join(SKILL_FILE);
    match std::fs::remove_file(&file) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Change::Absent(file)),
        Err(e) => return Err(e).with_context(|| format!("removing {}", file.display())),
    }
    let folder_is_empty = std::fs::read_dir(&dir).with_context(|| format!("reading {}", dir.display()))?.next().is_none();
    if folder_is_empty {
        std::fs::remove_dir(&dir).with_context(|| format!("removing {}", dir.display()))?;
    }
    Ok(Change::Removed(file))
}

fn describe(agent: &Agent, change: &Change) -> String {
    let label = agent.label;
    match change {
        Change::Written(file) => {
            let line = format!("installed the {label} skill at {}", file.display());
            match agent.written_note {
                Some(note) => format!("{line}\n{note}"),
                None => line,
            }
        }
        Change::Skipped(config) => format!("skipped the {label} skill: {} does not exist", config.display()),
        Change::Removed(file) => format!("removed the {label} skill at {}", file.display()),
        Change::Absent(file) => format!("no {label} skill at {}", file.display()),
    }
}

fn agent_config(agent: &Agent) -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not resolve the home directory")?;
    Ok(config_dir(agent, std::env::var_os(agent.config_env), &home))
}

pub fn install() -> Result<()> {
    let exe = std::env::current_exe().context("current_exe")?;
    for agent in &AGENTS {
        let change = write_skill(agent, &agent_config(agent)?, &exe)?;
        println!("{}", describe(agent, &change));
    }
    Ok(())
}

pub fn uninstall() -> Result<()> {
    for agent in &AGENTS {
        let change = remove_skill(&agent_config(agent)?)?;
        println!("{}", describe(agent, &change));
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/install/skills/mod.rs"]
mod tests;
