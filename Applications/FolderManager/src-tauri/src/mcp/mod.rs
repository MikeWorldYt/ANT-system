// The `--mcp [storage]` mode: a stdio MCP server that lets an agent read the storage through the app's own services.
mod protocol;
mod schema;
#[cfg(test)]
mod tests;
mod tools;
mod view;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use crate::app::storage_session::Storage;
use crate::domain::error::{FmError, R};
use crate::infra::storage::pointer;
use crate::services::watch::local_data_dir;

const NO_STORAGE: &str = "No storage given: start the server with `--mcp <storage path>`, or open a storage in Folder Manager once so it is remembered";

pub struct Server {
    storage: Result<Storage, String>,
}

impl Server {
    pub fn open(arg: Option<PathBuf>, remembered: impl FnOnce() -> Option<PathBuf>) -> Self {
        Self { storage: arg.or_else(remembered).map(Storage::resolve).ok_or_else(|| NO_STORAGE.to_string()) }
    }

    // Reads never take the storage lock, so an agent's read neither waits for nor holds up a change in the app.
    fn read<T: Serialize>(&self, f: impl FnOnce(&Storage, &Path) -> R<T>) -> R<Value> {
        let storage = self.storage.as_ref().map_err(|message| FmError::Internal { message: message.clone() })?;
        let value = f(storage, &storage.root()?)?;
        serde_json::to_value(value).map_err(|e| FmError::Internal { message: e.to_string() })
    }
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
