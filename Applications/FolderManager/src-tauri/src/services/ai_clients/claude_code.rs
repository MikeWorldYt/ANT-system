// Drives the Claude Code CLI, since ~/.claude.json is rewritten by running sessions and must not be edited.
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use super::{exe_path, internal, server_entry, AiClient, SERVER_NAME};
use crate::domain::error::R;

fn find_cli() -> Option<PathBuf> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let on_path = std::env::split_paths(&path).flat_map(|dir| [dir.join("claude.exe"), dir.join("claude.cmd")]).find(|p| p.is_file());
    on_path.or_else(|| {
        let home = std::env::var_os("USERPROFILE").map(PathBuf::from)?;
        Some(home.join(".local").join("bin").join("claude.exe")).filter(|p| p.is_file())
    })
}

fn run(cli: &Path, args: &[&str]) -> R<Output> {
    let mut cmd = Command::new(cli);
    cmd.args(args);
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut cmd, crate::platform::windows::powershell::CREATE_NO_WINDOW);
    cmd.output().map_err(|e| internal(format!("Could not run the Claude Code CLI: {e}")))
}

fn run_checked(cli: &Path, args: &[&str]) -> R<()> {
    let out = run(cli, args)?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let detail = if stderr.trim().is_empty() { stdout.trim() } else { stderr.trim() };
    Err(internal(format!("claude {} failed: {detail}", args[..2].join(" "))))
}

fn is_connected(cli: &Path) -> R<bool> {
    Ok(run(cli, &["mcp", "get", SERVER_NAME])?.status.success())
}

fn client(cli: Option<&Path>, connected: bool) -> AiClient {
    AiClient {
        id: "claudeCode",
        name: "Claude Code",
        installed: cli.is_some(),
        connected,
        config_path: cli.map(|c| c.display().to_string()),
        restart_needed: false,
    }
}

pub(super) fn status() -> R<AiClient> {
    let cli = find_cli();
    let connected = match &cli {
        Some(cli) => is_connected(cli)?,
        None => false,
    };
    Ok(client(cli.as_deref(), connected))
}

pub(super) fn required_cli() -> R<PathBuf> {
    find_cli().ok_or_else(|| internal("Claude Code is not installed"))
}

fn remove(cli: &Path) -> R<()> {
    run_checked(cli, &["mcp", "remove", SERVER_NAME, "-s", "user"])
}

pub(super) fn connect() -> R<AiClient> {
    let cli = required_cli()?;
    if is_connected(&cli)? {
        remove(&cli)?;
    }
    let json = server_entry(&exe_path()?).to_string();
    run_checked(&cli, &["mcp", "add-json", SERVER_NAME, &json, "-s", "user"])?;
    Ok(client(Some(&cli), is_connected(&cli)?))
}

pub(super) fn disconnect() -> R<AiClient> {
    let cli = required_cli()?;
    if is_connected(&cli)? {
        remove(&cli)?;
    }
    Ok(client(Some(&cli), is_connected(&cli)?))
}
