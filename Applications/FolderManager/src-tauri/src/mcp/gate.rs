// An agent's writes stay off until the user turns them on for the storage, and deletes, moves and batches wait for the user's yes in the open window.
use std::path::Path;
use std::time::Duration;

use serde_json::Value;

use super::Server;
use crate::domain::error::{FmError, R};
use crate::infra::storage::db::Store;
use crate::services::bridge::{client, UiCall};

pub const WRITES_KEY: &str = "mcpWrites";
pub const MAX_BATCH: usize = 50;
pub const MAX_FILES: usize = 500;
pub const MAX_ENTRIES: usize = 500;
pub const CONFIRM_WAIT: Duration = Duration::from_secs(60);

const READ_ONLY: &str = "Folder Manager lets AI agents only read this storage, so nothing was changed. To allow changes, ask the user to turn on \"Let AI agents change this storage\" in Folder Manager under Settings > Advanced, then call the tool again.";
const NO_APP: &str = "This change needs the user's OK in the Folder Manager window, and the app is not running, so nothing was changed. Ask the user to open Folder Manager on this storage, then call the tool again.";
const DENIED: &str = "The user declined this change in the Folder Manager window, so nothing was changed. Do not try it again unless the user asks.";

#[derive(Clone, Copy)]
pub enum Gate {
    App,
    #[cfg(test)]
    Open,
}

fn refused(message: String) -> FmError {
    FmError::Internal { message }
}

// Read on every call from ant.db, where the app keeps its settings, so turning it off stops the next call; no db means off.
pub fn writes_on(root: &Path) -> R<bool> {
    let Some(store) = Store::open_existing(root)? else { return Ok(false) };
    Ok(store.settings()?.get(WRITES_KEY).is_some_and(|v| v == "true"))
}

impl Server {
    // Asked before the mutation takes the storage lock, so a person thinking it over never holds up the app.
    pub(super) fn allow(&self, root: &Path, title: &str, confirm: bool, items: Vec<String>) -> R<()> {
        if !matches!(self.gate, Gate::App) {
            return Ok(());
        }
        if !writes_on(root)? {
            return Err(refused(READ_ONLY.to_string()));
        }
        if !confirm {
            return Ok(());
        }
        let file = self.bridge.as_ref().map_err(|_| refused(NO_APP.to_string()))?;
        let call = UiCall::Confirm { title: title.to_string(), items, seconds: self.confirm_wait.as_secs() };
        match client::send(file, root, call) {
            Ok(reply) if reply.get("confirmed") == Some(&Value::Bool(true)) => Ok(()),
            Ok(_) => Err(refused(DENIED.to_string())),
            Err(message) if message == client::NOT_RUNNING => Err(refused(NO_APP.to_string())),
            Err(message) => Err(refused(format!("The user did not confirm this change in the Folder Manager window, so nothing was changed ({message}). Call the tool again when the user is at the window."))),
        }
    }
}
