// The local channel through which an MCP process asks the running app to move its interface, one line of JSON each way.
pub mod client;
pub mod endpoint;
pub mod server;
#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

// The webview gets this long to act and answer; the MCP side waits REPLY_MARGIN longer so the app's own timeout reaches it.
pub const ANSWER_TIMEOUT: Duration = Duration::from_secs(5);
const REPLY_MARGIN: Duration = Duration::from_secs(3);
const MAX_CONFIRM: u64 = 120;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "action", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum UiCall {
    GetState,
    Reveal { rel_parts: Vec<String> },
    OpenTab { rel_parts: Option<Vec<String>> },
    Split { rel_parts: Option<Vec<String>> },
    SetView { view: String },
    Confirm { title: String, items: Vec<String>, seconds: u64 },
}

impl UiCall {
    // A person answers a confirmation; every other call the window answers by itself.
    pub fn wait(&self) -> Duration {
        match self {
            Self::Confirm { seconds, .. } => Duration::from_secs((*seconds).clamp(1, MAX_CONFIRM)),
            _ => ANSWER_TIMEOUT,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    token: String,
    root: PathBuf,
    call: UiCall,
}

pub type Reply = Result<Value, String>;
