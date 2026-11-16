// The `--mcp [storage]` mode: a stdio MCP server that lets an agent read and change the storage through the app's own services.
mod guide;
mod protocol;
mod retry;
mod schema;
#[cfg(test)]
mod tests;
mod tools;
mod ui;
mod ui_schema;
#[cfg(test)]
mod ui_tests;
mod view;
mod write;
mod write_schema;
#[cfg(test)]
mod write_tests;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use retry::{Retry, RETRY};
use crate::app::storage_session::Storage;
use crate::domain::error::{FmError, R};
use crate::infra::storage::pointer;
use crate::platform::recycle::send_to_recycle_bin;
use crate::services::watch::local_data_dir;

const NO_STORAGE: &str = "No storage given: start the server with `--mcp <storage path>`, or open a storage in Folder Manager once so it is remembered";

pub struct Server {
    storage: Result<Storage, String>,
    recycle: fn(&Path) -> Result<(), String>,
    retry: Retry,
    bridge: Result<PathBuf, String>,
}

impl Server {
    pub fn open(arg: Option<PathBuf>, remembered: impl FnOnce() -> Option<PathBuf>) -> Self {
        let storage = arg.or_else(remembered).map(Storage::resolve).ok_or_else(|| NO_STORAGE.to_string());
        Self { storage, recycle: send_to_recycle_bin, retry: RETRY, bridge: ui::bridge_file() }
    }

    fn storage(&self) -> R<&Storage> {
        self.storage.as_ref().map_err(|message| FmError::Internal { message: message.clone() })
    }

    // Reads never take the storage lock, so an agent's read neither waits for nor holds up a change in the app.
    fn read<T: Serialize>(&self, f: impl FnOnce(&Storage, &Path) -> R<T>) -> R<Value> {
        let storage = self.storage()?;
        json(f(storage, &storage.root()?)?)
    }
}

fn json<T: Serialize>(value: T) -> R<Value> {
    serde_json::to_value(value).map_err(|e| FmError::Internal { message: e.to_string() })
}

fn remembered() -> Option<PathBuf> {
    match local_data_dir() {
        Ok(dir) => pointer::remembered(&dir),
        Err(e) => {
            eprintln!("MCP: no remembered storage: {e}");
            None
        }
    }
}

// stdout carries the protocol only, so every message for a person goes to stderr.
pub fn run_headless(arg: Option<OsString>) -> i32 {
    let server = Server::open(arg.map(PathBuf::from), remembered);
    match server.storage.as_ref().map(Storage::root) {
        Ok(Ok(root)) => eprintln!("MCP: serving {}", root.display()),
        Ok(Err(e)) => eprintln!("MCP: {e}"),
        Err(message) => eprintln!("MCP: {message}"),
    }
    match protocol::serve(&server, std::io::stdin().lock(), std::io::stdout().lock()) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("MCP: stopped: {e}");
            1
        }
    }
}
