// Reads and writes OpenCode's global opencode.json, the file it loads MCP servers from.
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use super::shared::{home_dir, io_err, on_path, read_text, same_exe, write_checked};
use super::{exe_path, internal, AiClient, SERVER_NAME};
use crate::domain::error::{FmError, R};

const FILES: [&str; 2] = ["opencode.json", "opencode.jsonc"];

fn entry(exe: &str) -> Value {
    json!({ "type": "local", "command": [exe, "--mcp"], "enabled": true })
}

pub(crate) fn config_dir(xdg_config: Option<&Path>, home: Option<&Path>) -> Option<PathBuf> {
    xdg_config.map(|x| x.join("opencode")).or_else(|| home.map(|h| h.join(".config").join("opencode")))
}

fn current_dir() -> Option<PathBuf> {
    let xdg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).filter(|p| p.is_absolute());
    config_dir(xdg.as_deref(), home_dir().as_deref())
}

// A .jsonc that holds comments fails the JSON parse and is refused, so only an existing file is ever picked over a new opencode.json.
fn config_file(dir: &Path) -> PathBuf {
    FILES.iter().map(|f| dir.join(f)).find(|p| p.is_file()).unwrap_or_else(|| dir.join(FILES[0]))
}

fn read_config(file: &Path) -> R<(Map<String, Value>, Option<String>)> {
    let text = read_text(file)?;
    let Some(body) = text.as_deref().filter(|t| !t.trim().is_empty()) else { return Ok((Map::new(), text)) };
    match serde_json::from_str::<Value>(body) {
        Ok(Value::Object(map)) => Ok((map, text)),
        Ok(_) => Err(internal(format!("{} is not a JSON object; it was left untouched", file.display()))),
        Err(e) => Err(internal(format!("{} is not plain JSON ({e}); it was left untouched", file.display()))),
    }
}

fn write_config(file: &Path, read: Option<&str>, config: &Map<String, Value>) -> R<()> {
    write_checked(file, read, &serde_json::to_string_pretty(config).map_err(|e| internal(e.to_string()))?)
}

fn not_object(file: &Path) -> FmError {
    internal(format!("\"mcp\" in {} is not an object; it was left untouched", file.display()))
}

pub(crate) fn connect_in(dir: &Path, exe: &str) -> R<()> {
    let file = config_file(dir);
    let (mut config, read) = read_config(&file)?;
    let Value::Object(servers) = config.entry("mcp").or_insert_with(|| Value::Object(Map::new())) else {
        return Err(not_object(&file));
    };
    servers.insert(SERVER_NAME.into(), entry(exe));
    fs::create_dir_all(dir).map_err(|e| io_err("create", dir, e))?;
    write_config(&file, read.as_deref(), &config)
}

pub(crate) fn disconnect_in(dir: &Path) -> R<()> {
    let file = config_file(dir);
    let (mut config, read) = read_config(&file)?;
    let removed = match config.get_mut("mcp") {
        None => false,
        Some(Value::Object(servers)) => servers.remove(SERVER_NAME).is_some(),
        Some(_) => return Err(not_object(&file)),
    };
    if removed { write_config(&file, read.as_deref(), &config) } else { Ok(()) }
}

pub(crate) fn is_connected_in(dir: &Path, exe: &str) -> R<bool> {
    let (config, _) = read_config(&config_file(dir))?;
    let command = config.get("mcp").and_then(|s| s.get(SERVER_NAME)).and_then(|e| e.get("command")).and_then(|c| c.get(0)).and_then(Value::as_str);
    Ok(command.is_some_and(|c| same_exe(c, exe)))
}

fn installed_dir() -> Option<PathBuf> {
    current_dir().filter(|d| d.is_dir())
}

// A config the dialog cannot parse must not hide every other client, so it reads as not connected; connect reports the refusal.
pub(super) fn status() -> R<AiClient> {
    let dir = installed_dir();
    let installed = dir.is_some() || on_path(&["opencode.exe", "opencode.cmd"]);
    let file = current_dir().map(|d| config_file(&d));
    let connected = match (&dir, exe_path()) {
        (Some(dir), Ok(exe)) => is_connected_in(dir, &exe).unwrap_or(false),
        _ => false,
    };
    Ok(AiClient { id: "opencode", name: "OpenCode", installed, connected, config_path: file.map(|f| f.display().to_string()), restart_needed: false })
}

fn required_dir() -> R<PathBuf> {
    current_dir().ok_or_else(|| internal("Could not locate the OpenCode config folder"))
}

pub(super) fn connect() -> R<AiClient> {
    connect_in(&required_dir()?, &exe_path()?)?;
    status()
}

pub(super) fn disconnect() -> R<AiClient> {
    disconnect_in(&required_dir()?)?;
    status()
}
