// File tools beyond the tree: copies, shortcuts outside the storage, and the user's Downloads folder, which the app browses under `::Downloads`.
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{json, Value};

use super::gate::MAX_FILES;
use super::protocol::{RpcError, INVALID_PARAMS};
use super::schema::{object, output, rel_parts, strings};
use super::tools::args;
use super::write::{indexed, source};
use super::Server;
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::validate_component;
use crate::platform::shortcuts::create_shortcuts;
use crate::services::tree::tree_store;
use crate::services::{downloads, mutations};

type Schemas = fn() -> (Value, Value);

const TOOLS: [(&str, &str, &str, Schemas, bool); 5] = [
    ("copy_files", "Copy files", "Copies files (or plain folders) into a folder, as a Ctrl drop in the app; sources may be under `::Downloads`.", copy_files, false),
    ("create_shortcut", "Create shortcut", "Puts a shortcut to a storage folder or file in a folder outside the storage, e.g. the Desktop, under a free name.", create_shortcut, false),
    ("list_downloads", "List Downloads", "The files and folders in the user's Downloads folder, or in a folder below it.", list_downloads, false),
    ("rename_download", "Rename download", "Renames a file or folder in the Downloads folder; a name already taken is refused.", rename_download, false),
    ("delete_downloads", "Delete downloads", "Sends files or folders of the Downloads folder to the Recycle Bin, never deleting them for good.", delete_downloads, true),
];

pub(super) fn has(name: &str) -> bool {
    TOOLS.iter().any(|t| t.0 == name)
}

pub(super) fn list() -> Vec<Value> {
    TOOLS
        .iter()
        .map(|(name, title, description, schemas, destructive)| {
            let (input, out) = schemas();
            let annotations = match *name {
                "list_downloads" => json!({ "title": title, "readOnlyHint": true, "openWorldHint": false }),
                _ => json!({ "title": title, "readOnlyHint": false, "destructiveHint": destructive, "idempotentHint": false, "openWorldHint": false }),
            };
            json!({ "name": name, "title": title, "description": description, "inputSchema": input, "outputSchema": out, "annotations": annotations })
        })
        .collect()
}

fn done() -> Value {
    output(json!({ "changed": { "type": "string" }, "warnings": strings() }), &["changed"])
}

fn downloads_path() -> Value {
    json!({ "type": "array", "minItems": 1, "items": { "type": "string" }, "description": "Starts with \"::Downloads\", then the names below it" })
}

fn copy_files() -> (Value, Value) {
    let input = object(json!({ "sources": { "type": "array", "minItems": 1, "items": rel_parts(1) }, "destination": rel_parts(1) }), &["sources", "destination"]);
    (input, done())
}

fn create_shortcut() -> (Value, Value) {
    let input = object(json!({ "target": rel_parts(1), "folder": { "type": "string", "minLength": 1, "description": "Absolute path of a folder outside the storage" } }), &["target", "folder"]);
    (input, done())
}

fn list_downloads() -> (Value, Value) {
    (object(json!({ "relParts": downloads_path() }), &["relParts"]), output(json!({ "nodes": { "type": "array" } }), &["nodes"]))
}

fn rename_download() -> (Value, Value) {
    (object(json!({ "relParts": downloads_path(), "name": { "type": "string", "minLength": 1 } }), &["relParts", "name"]), done())
}

fn delete_downloads() -> (Value, Value) {
    (object(json!({ "targets": { "type": "array", "minItems": 1, "items": downloads_path() } }), &["targets"]), done())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CopyArgs {
    sources: Vec<Vec<String>>,
    destination: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ShortcutArgs {
    target: Vec<String>,
    folder: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DownloadArgs {
    rel_parts: Vec<String>,
    name: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TargetsArgs {
    targets: Vec<Vec<String>>,
}

// Each name is validated below the Downloads folder, so no call can reach outside it.
fn in_downloads(rel: &[String]) -> R<PathBuf> {
    match rel.split_first() {
        Some((first, rest)) if first == downloads::ROOT => downloads::path_of(&downloads::system_base()?, rest),
        _ => Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "a Downloads path starts with \"::Downloads\"".to_string() }),
    }
}

fn shortcut(root: &Path, target: &[String], folder: &Path) -> R<String> {
    let (abs, link) = if root.join(target.join("/")).is_file() {
        mutations::link_in(root, mutations::file_path(root, target)?, folder)?
    } else {
        mutations::shortcut_plan(root, &tree_store::load_paths(root, &[target.to_vec()])?, target, folder)?
    };
    let failures = create_shortcuts(&[(abs, link.clone())]);
    if !failures.is_empty() {
        return Err(FmError::Internal { message: failures.join("; ") });
    }
    Ok(link.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())
}

impl Server {
    // Downloads and shortcuts sit outside the storage, so no lock or index step applies; the writes switch and the window's OK still do.
    fn outside(&self, title: &str, confirm: bool, items: Vec<String>, f: impl FnOnce(&Path) -> R<String>) -> R<Value> {
        let root = self.storage()?.root()?;
        self.allow(&root, title, confirm, items)?;
        let changed = f(&root)?;
        Ok(json!({ "changed": changed, "warnings": [] }))
    }
}

pub(super) fn call(server: &Server, name: &str, arguments: &Value) -> Result<R<Value>, RpcError> {
    Ok(match name {
        "copy_files" => {
            let a: CopyArgs = args(name, arguments)?;
            if a.sources.len() > MAX_FILES {
                return Err((INVALID_PARAMS, format!("Invalid arguments for {name}: {} files in one call, at most {MAX_FILES}; nothing was changed", a.sources.len())));
            }
            let items: Vec<String> = a.sources.iter().map(|s| format!("{} → {}", s.join("/"), a.destination.join("/"))).collect();
            let confirm = items.len() > 1;
            server.change_as("Copy files", confirm, items, format!("Copied {} items into {:?}", a.sources.len(), a.destination.join("/")), |root| {
                let paths = a.sources.iter().map(|rel| source(root, rel)).collect::<R<Vec<_>>>()?;
                indexed(root, |root| Ok(mutations::move_in(root, &a.destination, &paths, true, &server.recycle)?.mutation))
            })
        }
        "create_shortcut" => {
            let a: ShortcutArgs = args(name, arguments)?;
            server.outside("Create shortcut", false, vec![format!("{} → {}", a.target.join("/"), a.folder)], |root| {
                shortcut(root, &a.target, Path::new(&a.folder)).map(|link| format!("Made the shortcut {link:?} in {:?}", a.folder))
            })
        }
        "list_downloads" => {
            let a: DownloadArgs = args(name, arguments)?;
            in_downloads(&a.rel_parts).and_then(|_| {
                let rest = &a.rel_parts[1..];
                let nodes = downloads::list(&downloads::system_base()?, rest)?;
                let rows: Vec<Value> = nodes.iter().filter(|n| !n.concealed).map(|n| json!({ "name": n.name, "relParts": n.rel_parts, "file": n.file })).collect();
                Ok(json!({ "nodes": rows }))
            })
        }
        "rename_download" => {
            let a: DownloadArgs = args(name, arguments)?;
            let new = a.name.clone().ok_or((INVALID_PARAMS, format!("Invalid arguments for {name}: missing field `name`")))?;
            server.outside("Rename download", false, vec![a.rel_parts.join("/")], |_| {
                let from = in_downloads(&a.rel_parts)?;
                let new = new.trim();
                validate_component(new)?;
                let to = from.with_file_name(new);
                let same = from.to_string_lossy().eq_ignore_ascii_case(&to.to_string_lossy());
                if to.exists() && !same {
                    return Err(FmError::AlreadyExists { name: new.to_string() });
                }
                std::fs::rename(&from, &to).map_err(|e| crate::infra::fs_safe::io_err("rename download", &from, &e))?;
                Ok(format!("Renamed {:?} to {new:?}", a.rel_parts.join("/")))
            })
        }
        "delete_downloads" => {
            let a: TargetsArgs = args(name, arguments)?;
            let items = a.targets.iter().map(|t| t.join("/")).collect();
            server.outside("Delete downloads", true, items, |_| {
                for rel in &a.targets {
                    let path = in_downloads(rel)?;
                    (server.recycle)(&path).map_err(|e| FmError::Internal { message: format!("Could not delete {}: {e}", path.display()) })?;
                }
                Ok(format!("Sent {} items of Downloads to the Recycle Bin", a.targets.len()))
            })
        }
        _ => return Err((INVALID_PARAMS, format!("Unknown tool: {name}"))),
    })
}
