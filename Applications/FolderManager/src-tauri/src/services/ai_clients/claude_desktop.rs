// Reads and writes claude_desktop_config.json, the only file Claude Desktop loads MCP servers from.
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use super::{exe_path, internal, server_entry, AiClient, SERVER_NAME};
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::{write_atomic, write_atomic_io};

const FILE: &str = "claude_desktop_config.json";
const BACKUP: &str = "claude_desktop_config.json.bak";

fn first_dir(parent: &Path, prefix: &str, sub: &[&str]) -> Option<PathBuf> {
    fs::read_dir(parent).ok()?.flatten().filter(|e| e.file_name().to_string_lossy().starts_with(prefix)).map(|e| sub.iter().fold(e.path(), |p, s| p.join(s))).find(|p| p.is_dir())
}

// Newer Store builds keep their data in Claude-Data; older ones virtualize %APPDATA% into the package folder.
pub(crate) fn config_dir(local_app_data: Option<&Path>, app_data: Option<&Path>) -> Option<PathBuf> {
    let store = local_app_data.and_then(|local| first_dir(&local.join("Claude-Data"), "claude_", &[]).or_else(|| first_dir(&local.join("Packages"), "Claude_", &["LocalCache", "Roaming", "Claude"])));
    store.or_else(|| app_data.map(|a| a.join("Claude")))
}

fn current_dir() -> Option<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let roaming = std::env::var_os("APPDATA").map(PathBuf::from);
    config_dir(local.as_deref(), roaming.as_deref()).filter(|d| d.is_dir())
}

fn io_err(op: &str, path: &Path, e: std::io::Error) -> FmError {
    FmError::Io { op: op.into(), path: path.display().to_string(), message: e.to_string(), os_code: e.raw_os_error() }
}

fn read_text(file: &Path) -> R<Option<String>> {
    match fs::read_to_string(file) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_err("read", file, e)),
    }
}

// Returns the text it parsed too, so a write can tell whether Claude Desktop changed the file meanwhile.
fn read_config(file: &Path) -> R<(Map<String, Value>, Option<String>)> {
    let text = read_text(file)?;
    let Some(body) = text.as_deref().filter(|t| !t.trim().is_empty()) else { return Ok((Map::new(), text)) };
    match serde_json::from_str::<Value>(body) {
        Ok(Value::Object(map)) => Ok((map, text)),
        Ok(_) => Err(internal(format!("{} is not a JSON object; it was left untouched", file.display()))),
        Err(e) => Err(internal(format!("{} is not valid JSON ({e}); it was left untouched", file.display()))),
    }
}

// Claude Desktop rewrites this file itself, so a change since the read wins and the user's previous file stays in the .bak.
pub(super) fn write_config(file: &Path, read: Option<&str>, config: &Map<String, Value>) -> R<()> {
    let text = serde_json::to_string_pretty(config).map_err(|e| internal(e.to_string()))?;
    if read_text(file)?.as_deref() != read {
        return Err(internal(format!("{} changed while Folder Manager was updating it; nothing was written, try again", file.display())));
    }
    if let Some(previous) = read.filter(|t| !t.trim().is_empty()) {
        let backup = file.with_file_name(BACKUP);
        write_atomic_io(&backup, previous.as_bytes()).map_err(|e| io_err("back up", &backup, e))?;
    }
    write_atomic(file, text.as_bytes())
}

fn not_object(file: &Path) -> FmError {
    internal(format!("\"mcpServers\" in {} is not an object; it was left untouched", file.display()))
}

pub(crate) fn connect_in(dir: &Path, exe: &str) -> R<()> {
    let file = dir.join(FILE);
    let (mut config, read) = read_config(&file)?;
    let Value::Object(servers) = config.entry("mcpServers").or_insert_with(|| Value::Object(Map::new())) else {
        return Err(not_object(&file));
    };
    servers.insert(SERVER_NAME.into(), server_entry(exe));
    fs::create_dir_all(dir).map_err(|e| io_err("create", dir, e))?;
    write_config(&file, read.as_deref(), &config)
}

pub(crate) fn disconnect_in(dir: &Path) -> R<()> {
    let file = dir.join(FILE);
    let (mut config, read) = read_config(&file)?;
    let removed = match config.get_mut("mcpServers") {
        None => false,
        Some(Value::Object(servers)) => servers.remove(SERVER_NAME).is_some(),
        Some(_) => return Err(not_object(&file)),
    };
    if removed { write_config(&file, read.as_deref(), &config) } else { Ok(()) }
}

fn same_exe(a: &str, b: &str) -> bool {
    if cfg!(windows) { a.eq_ignore_ascii_case(b) } else { a == b }
}

pub(crate) fn is_connected_in(dir: &Path, exe: &str) -> R<bool> {
    let (config, _) = read_config(&dir.join(FILE))?;
    let command = config.get("mcpServers").and_then(|s| s.get(SERVER_NAME)).and_then(|e| e.get("command")).and_then(Value::as_str);
    Ok(command.is_some_and(|c| same_exe(c, exe)))
}

pub(super) fn status(restart_needed: bool) -> R<AiClient> {
    let dir = current_dir();
    let connected = match &dir {
        Some(dir) => is_connected_in(dir, &exe_path()?)?,
        None => false,
    };
    Ok(AiClient {
        id: "claudeDesktop",
        name: "Claude Desktop",
        installed: dir.is_some(),
        connected,
        config_path: dir.map(|d| d.join(FILE).display().to_string()),
        restart_needed,
    })
}

fn required_dir() -> R<PathBuf> {
    current_dir().ok_or_else(|| internal("Claude Desktop is not installed"))
}

pub(super) fn connect() -> R<AiClient> {
    connect_in(&required_dir()?, &exe_path()?)?;
    status(true)
}

pub(super) fn disconnect() -> R<AiClient> {
    disconnect_in(&required_dir()?)?;
    status(true)
}
