// Connects the app's own MCP server (`<exe> --mcp`) to the user's AI clients.
mod claude_code;
mod claude_desktop;
mod restart;
#[cfg(test)]
mod tests;

use serde::Serialize;
use serde_json::{json, Value};

use crate::domain::error::{FmError, R};

pub const SERVER_NAME: &str = "ant-system";

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiClient {
    pub id: &'static str,
    pub name: &'static str,
    pub installed: bool,
    pub connected: bool,
    pub config_path: Option<String>,
    pub restart_needed: bool,
}

pub(crate) fn internal(message: impl Into<String>) -> FmError {
    FmError::Internal { message: message.into() }
}

fn exe_path() -> R<String> {
    let exe = std::env::current_exe().map_err(|e| internal(format!("Could not locate this app: {e}")))?;
    Ok(exe.to_string_lossy().into_owned())
}

pub(crate) fn server_entry(exe: &str) -> Value {
    json!({ "command": exe, "args": ["--mcp"] })
}

pub fn list() -> R<Vec<AiClient>> {
    Ok(vec![claude_desktop::status(false)?, claude_code::status()?])
}

pub fn connect(id: &str) -> R<AiClient> {
    match id {
        "claudeDesktop" => claude_desktop::connect(),
        "claudeCode" => claude_code::connect(),
        other => Err(internal(format!("Unknown AI client \"{other}\""))),
    }
}

pub fn disconnect(id: &str) -> R<AiClient> {
    match id {
        "claudeDesktop" => claude_desktop::disconnect(),
        "claudeCode" => claude_code::disconnect(),
        other => Err(internal(format!("Unknown AI client \"{other}\""))),
    }
}

pub fn restart(id: &str) -> R<AiClient> {
    match id {
        "claudeDesktop" => restart::restart_claude_desktop(),
        other => Err(internal(format!("Restart is not available for \"{other}\""))),
    }
}

pub fn config_snippet() -> R<String> {
    let doc = json!({ "mcpServers": { SERVER_NAME: server_entry(&exe_path()?) } });
    serde_json::to_string_pretty(&doc).map_err(|e| internal(e.to_string()))
}
