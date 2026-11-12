// The interface tools: the only ones that move the open window, each sent to the running app over the UI bridge.
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{json, Value};

use super::protocol::{RpcError, INVALID_PARAMS};
use super::tools::{args, node};
use super::ui_schema::{self as schema, VIEWS};
use super::Server;
use crate::domain::error::{FmError, R};
use crate::services::bridge::{client, endpoint, UiCall};
use crate::services::watch::local_data_dir;

type Schemas = fn() -> (Value, Value);

const TOOLS: [(&str, &str, &str, Schemas); 5] = [
    ("ui_get_state", "Window state", "What the open Folder Manager window shows: its storage, tabs, split, view and selection. Moves nothing.", schema::get_state),
    ("ui_reveal", "Reveal in window", "Selects a folder or file in the window's active pane, expanding the folders above it.", schema::reveal),
    ("ui_open_tab", "Open tab", "Opens a new tab in the window on a folder, or on the storage root without relParts.", schema::place),
    ("ui_split", "Open split", "Opens the split pane; with relParts it opens that folder there and focuses it.", schema::place),
    ("ui_set_view", "Set view", "Switches the window's active pane to another view.", schema::set_view),
];

pub(super) fn list() -> Vec<Value> {
    TOOLS
        .iter()
        .map(|(name, title, description, schemas)| {
            let (input, output) = schemas();
            let annotations = match *name {
                "ui_get_state" => json!({ "title": title, "readOnlyHint": true, "openWorldHint": false }),
                _ => json!({ "title": title, "readOnlyHint": false, "destructiveHint": false, "idempotentHint": *name != "ui_open_tab", "openWorldHint": false }),
            };
            json!({ "name": name, "title": title, "description": description, "inputSchema": input, "outputSchema": output, "annotations": annotations })
        })
        .collect()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NoArgs {}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PathArgs {
    rel_parts: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlaceArgs {
    rel_parts: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ViewArgs {
    view: String,
}

// An empty path is the storage root, which the window opens when given none.
fn place(a: PlaceArgs) -> Option<Vec<String>> {
    a.rel_parts.filter(|p| !p.is_empty())
}

pub(super) fn call(server: &Server, name: &str, arguments: &Value) -> Result<R<Value>, RpcError> {
    let call = match name {
        "ui_get_state" => args::<NoArgs>(name, arguments).map(|_| UiCall::GetState)?,
        "ui_reveal" => UiCall::Reveal { rel_parts: args::<PathArgs>(name, arguments)?.rel_parts },
        "ui_open_tab" => UiCall::OpenTab { rel_parts: place(args(name, arguments)?) },
        "ui_split" => UiCall::Split { rel_parts: place(args(name, arguments)?) },
        "ui_set_view" => {
            let a: ViewArgs = args(name, arguments)?;
            if !VIEWS.contains(&a.view.as_str()) {
                return Err((INVALID_PARAMS, format!("Invalid arguments for {name}: view must be one of {}", VIEWS.join(", "))));
            }
            UiCall::SetView { view: a.view }
        }
        _ => return Err((INVALID_PARAMS, format!("Unknown tool: {name}"))),
    };
    Ok(server.ui(call))
}

pub(super) fn bridge_file() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    Ok(endpoint::file(&local_data_dir().map_err(|e| e.to_string())?, &exe))
}

impl Server {
    fn ui(&self, call: UiCall) -> R<Value> {
        let root = self.storage()?.root()?;
        check(&root, &call)?;
        let file = self.bridge.as_ref().map_err(|message| FmError::Internal { message: message.clone() })?;
        client::send(file, &root, call).map_err(|message| FmError::Internal { message })
    }
}

// Checked here as the other tools check their paths, so the window never gets one outside the storage or missing from it.
fn check(root: &Path, call: &UiCall) -> R<()> {
    let (parts, folder) = match call {
        UiCall::Reveal { rel_parts } => (Some(rel_parts), false),
        UiCall::OpenTab { rel_parts } | UiCall::Split { rel_parts } => (rel_parts.as_ref(), true),
        UiCall::GetState | UiCall::SetView { .. } => (None, false),
    };
    let Some(parts) = parts else { return Ok(()) };
    let found = node(root, parts.clone())?;
    if folder && found.node.file {
        return Err(FmError::InvalidPath { rel_parts: parts.clone(), reason: "a file opens in no tab or split; reveal it instead".to_string() });
    }
    Ok(())
}
