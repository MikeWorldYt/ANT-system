// Where the running app tells MCP processes how to reach it: its port and session token, in the user's local app data.
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::infra::storage::db::hash;
use crate::platform::random;

const DIR: &str = "ui-bridge";

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Endpoint {
    pub port: u16,
    pub token: String,
}

// One file per copy of the executable, as the storage pointer, so `--mcp` finds the app it was started next to.
pub fn file(data_dir: &Path, exe: &Path) -> PathBuf {
    data_dir.join(DIR).join(format!("{}.json", hash(&exe.to_string_lossy())))
}

pub fn new_token() -> std::io::Result<String> {
    random::hex_token(32)
}

pub fn write(file: &Path, endpoint: &Endpoint) -> std::io::Result<()> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(file, serde_json::to_string(endpoint)?)
}

pub fn read(file: &Path) -> Option<Endpoint> {
    serde_json::from_str(&fs::read_to_string(file).ok()?).ok()
}

// A second window of the same executable may have written its own endpoint since, and keeps it.
pub fn remove(file: &Path, token: &str) {
    if read(file).is_some_and(|e| e.token == token) {
        if let Err(e) = fs::remove_file(file) {
            eprintln!("Could not remove the UI bridge file {}: {e}", file.display());
        }
    }
}
