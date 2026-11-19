// The write tools: each runs the mutation and index step its IPC command in `commands/edit.rs` runs, and answers what changed.
use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use super::protocol::{RpcError, INVALID_PARAMS};
use super::tools::args;
use super::view::Change;
use super::retry::retrying;
use super::gate::{MAX_BATCH, MAX_ENTRIES, MAX_FILES};
use super::{write_schema as schema, Server};
use crate::domain::config::{EditableType, NewKind, WkbStatus};
use crate::domain::error::R;
use crate::infra::storage::changes;
use crate::platform::shortcuts::create_shortcuts;
use crate::services::automations::dictionary::{self, Spec};
use crate::services::downloads;
use crate::services::edit;
use crate::services::index::{review_index, ReviewIndexEvent};
use crate::services::mutations::{self, BoardCard, MoveDirection, Mutation, ProjectPlan};

type Schemas = fn() -> (Value, Value);

#[derive(Clone, Copy)]
enum Kind {
    Read,
    Change,
    Idempotent,
    Destructive,
}

const TOOLS: [(&str, &str, &str, Schemas, Kind); 24] = [
    ("add_node", "Add folder", "Creates a folder inside `parent` (an empty parent adds a root category) with the given name.", schema::add_node, Kind::Change),
    ("add_folder", "Add plain folder", "Creates plain folders inside a worknode, untracked and without an id, e.g. `assets/kit`; anywhere else use add_node.", schema::add_folder, Kind::Change),
    ("add_worknode", "Add worknode", "Creates a worknode inside `parent` with the given name; the app numbers it.", schema::add_worknode, Kind::Change),
    ("rename", "Rename folder", "Renames a folder; its linked support and review folders follow.", schema::rename, Kind::Change),
    ("move", "Move folder", "Moves a folder into `destination`, where it takes the next free id.", schema::move_folder, Kind::Destructive),
    ("copy", "Copy folder", "Copies a folder into `destination`, keeping its id under the next free \"(n)\" name.", schema::move_folder, Kind::Change),
    ("reorder", "Move up or down", "Swaps a folder's id with its sibling above or below.", schema::reorder, Kind::Destructive),
    ("delete", "Delete folder", "Sends a folder to the Recycle Bin, never deleting it for good; a non-empty folder needs confirmNonEmpty.", schema::delete, Kind::Destructive),
    ("set_type", "Set type", "Makes a folder a NODE or a worknode (WKB), optionally with a worknode status.", schema::set_type, Kind::Idempotent),
    ("set_status", "Set status", "Sets a worknode's status in its info.json.", schema::set_status, Kind::Idempotent),
    ("set_owner", "Set owner", "Sets a worknode's owner in its info.json.", schema::set_owner, Kind::Idempotent),
    ("set_favorite", "Set favorite", "Stars or unstars a folder.", schema::set_flag, Kind::Idempotent),
    ("set_important", "Set important", "Marks or unmarks a worknode as important.", schema::set_flag, Kind::Idempotent),
    ("hide", "Hide folder", "Hides or shows a folder below the root level.", schema::hide, Kind::Idempotent),
    ("set_board", "Place board cards", "Places worknodes in task board columns and orders.", schema::set_board, Kind::Idempotent),
    ("move_files", "Move files", "Moves files (or plain folders) into a folder, as a drop in the app: files dropped on a node land in its review folder. Sources may be in the storage or under `::Downloads`, as ui_get_state lists the selection.", schema::move_files, Kind::Destructive),
    ("set_output", "Set output", "Sets the folder or file inside a worknode that is its deliverable (its `output` in info.json), or clears it with null.", schema::set_output, Kind::Idempotent),
    ("rename_file", "Rename file", "Renames a file inside a storage folder.", schema::rename_file, Kind::Change),
    ("delete_file", "Delete file", "Sends a file to the Recycle Bin, never deleting it for good.", schema::delete_file, Kind::Destructive),
    ("preview_project_groups", "Preview project sort", "The projects a review folder's loose files would be sorted into; changes nothing.", schema::preview_project_groups, Kind::Read),
    ("apply_project_groups", "Sort into projects", "Moves a review folder's loose files into new worknodes, as previewed or adjusted.", schema::apply_project_groups, Kind::Destructive),
    ("get_dictionary", "Read dictionary", "A folder's own `.ant/dictionary.md` and the entries in force there, inherited from the folders above; changes nothing.", schema::get_dictionary, Kind::Read),
    ("set_dictionary", "Write dictionary", "Writes a folder's `.ant/dictionary.md` from structured entries, replacing it whole; the app lays out the file and its header. Read ant://guide/dictionary for what each key and heading does, and the current entries with get_dictionary first, then send them all back with the changes.", schema::set_dictionary, Kind::Destructive),
    ("undo_project_sort", "Undo project sort", "Puts the files of the last project sort back and removes the folders it made.", schema::undo_project_sort, Kind::Destructive),
];

pub(super) fn list() -> Vec<Value> {
    TOOLS
        .iter()
        .map(|(name, title, description, schemas, kind)| {
            let (input, output) = schemas();
            let annotations = match kind {
                Kind::Read => json!({ "title": title, "readOnlyHint": true, "openWorldHint": false }),
                _ => json!({
                    "title": title, "readOnlyHint": false, "destructiveHint": matches!(kind, Kind::Destructive),
                    "idempotentHint": matches!(kind, Kind::Idempotent), "openWorldHint": false,
                }),
            };
            json!({ "name": name, "title": title, "description": description, "inputSchema": input, "outputSchema": output, "annotations": annotations })
        })
        .collect()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AddArgs {
    parent: Vec<String>,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NodeArgs {
    rel_parts: Vec<String>,
    name: Option<String>,
    direction: Option<MoveDirection>,
    #[serde(rename = "type")]
    node_type: Option<EditableType>,
    status: Option<WkbStatus>,
    owner: Option<String>,
    on: Option<bool>,
    hidden: Option<bool>,
    #[serde(default)]
    confirm_non_empty: bool,
    #[serde(default)]
    expect_ghost: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FolderArgs {
    parent: Vec<String>,
    path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FilesArgs {
    sources: Vec<Vec<String>>,
    destination: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OutputArgs {
    rel_parts: Vec<String>,
    output: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MoveArgs {
    source: Vec<String>,
    destination: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BoardArgs {
    cards: Vec<BoardCard>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GroupArgs {
    folder: Vec<String>,
    #[serde(default)]
    plans: Option<Vec<ProjectPlan>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NoArgs {}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DictionaryArgs {
    rel_parts: Vec<String>,
    #[serde(default)]
    entries: Option<Vec<Spec>>,
}

// A missing list would otherwise read as an empty one and wipe or skip everything without a word.
fn required<T>(name: &str, field: &str, value: Option<T>) -> Result<T, RpcError> {
    value.ok_or_else(|| (INVALID_PARAMS, format!("{name}: `{field}` is required")))
}

// The app shows review index progress; here no one listens, and its failures already come back as warnings.
fn quiet(_: &ReviewIndexEvent) -> Result<(), String> {
    Ok(())
}

fn indexed(root: &Path, f: impl FnOnce(&Path) -> R<Mutation>) -> R<Mutation> {
    edit::indexed(root, &quiet, f)
}

fn search<'a>(f: &'a dyn Fn(&Path) -> R<Mutation>) -> impl Fn(&Path) -> R<Mutation> + 'a {
    move |root| edit::search_indexed(root, f)
}

// A source under `::Downloads` names a file in the user's Downloads folder; any other is a path in the storage.
fn source(root: &Path, rel: &[String]) -> R<std::path::PathBuf> {
    match rel.split_first() {
        Some((first, rest)) if first == downloads::ROOT => {
            let path = downloads::path_of(&downloads::system_base()?, rest)?;
            if !path.exists() {
                return Err(crate::domain::error::FmError::NotFound { rel_parts: rel.to_vec() });
            }
            Ok(path)
        }
        _ => mutations::source_path(root, rel),
    }
}

fn plain(rel: &[String]) -> String {
    rel.join("/")
}

// Names are the user's data: quoted as JSON strings, a name holding quotes or line breaks cannot pass for the server's own words.
fn quoted(text: &str) -> String {
    Value::String(text.to_string()).to_string()
}

fn path(rel: &[String]) -> String {
    quoted(&plain(rel))
}

fn needs<T>(tool: &str, field: &str, value: Option<T>) -> Result<T, RpcError> {
    value.ok_or_else(|| (INVALID_PARAMS, format!("Invalid arguments for {tool}: missing field `{field}`")))
}

fn limit(tool: &str, what: &str, count: usize, max: usize) -> Result<(), RpcError> {
    if count <= max {
        return Ok(());
    }
    Err((INVALID_PARAMS, format!("Invalid arguments for {tool}: {count} {what} in one call, at most {max}; nothing was changed, send them in smaller calls")))
}

// Destructive tools and any call that touches more than one item wait for the user's yes.
fn asks(name: &str, items: &[String]) -> (&'static str, bool) {
    let tool = TOOLS.iter().find(|t| t.0 == name);
    (tool.map_or("Change", |t| t.1), tool.is_some_and(|t| matches!(t.4, Kind::Destructive)) || items.len() > 1)
}

impl Server {
    // The mutation takes the cross-process lock itself, as in the app; the in-process guard only mirrors `commands::locked`.
    fn change(&self, name: &str, items: Vec<String>, changed: String, f: impl Fn(&Path) -> R<Mutation>) -> R<Value> {
        let (title, confirm) = asks(name, &items);
        self.change_as(title, confirm, items, changed, f)
    }

    pub(super) fn change_as(&self, title: &str, confirm: bool, items: Vec<String>, changed: String, f: impl Fn(&Path) -> R<Mutation>) -> R<Value> {
        let storage = self.storage()?;
        let root = storage.root()?;
        self.allow(&root, title, confirm, items)?;
        let _guard = storage.guard();
        let mut change = Change::of(changed, retrying(self.retry, || f(&root))?);
        // Logged after the retries, so a busy log never runs the mutation twice.
        if let Err(e) = changes::record(&root, &change.levels, change.full_tree) {
            change.warnings.push(format!("The open Folder Manager may show this change only after its next refresh: {e}"));
        }
        super::json(change)
    }
}

pub(super) fn call(server: &Server, name: &str, arguments: &Value) -> Result<R<Value>, RpcError> {
    Ok(match name {
        "add_node" | "add_worknode" => {
            let a: AddArgs = args(name, arguments)?;
            let kind = if name == "add_node" { NewKind::Node } else { NewKind::Worknode };
            let what = if name == "add_node" { "folder" } else { "worknode" };
            server.change(name, Vec::new(), format!("Added a {what} {} in {}", quoted(&a.name), quoted(&format!("/{}", plain(&a.parent)))), |root| {
                indexed(root, |root| mutations::add_node_named(root, &a.parent, kind, Some(&a.name)))
            })
        }
        "add_folder" => {
            let a: FolderArgs = args(name, arguments)?;
            let parts: Vec<String> = a.path.split(['/', '\\']).filter(|p| !p.is_empty()).map(str::to_string).collect();
            server.change(name, Vec::new(), format!("Added the folder {} in {}", quoted(&a.path), path(&a.parent)), |root| mutations::add_folder(root, &a.parent, &parts))
        }
        "move_files" => {
            let a: FilesArgs = args(name, arguments)?;
            limit(name, "files", a.sources.len(), MAX_FILES)?;
            let items = a.sources.iter().map(|s| format!("{} → {}", plain(s), plain(&a.destination))).collect();
            server.change(name, items, format!("Moved {} items into {}", a.sources.len(), path(&a.destination)), |root| {
                let paths = a.sources.iter().map(|rel| source(root, rel)).collect::<R<Vec<_>>>()?;
                indexed(root, |root| {
                    let outcome = mutations::move_in(root, &a.destination, &paths, false, &server.recycle)?;
                    let mut mutation = outcome.mutation;
                    mutation.warnings.extend(outcome.conflicts.iter().map(|c| format!("{} is already in the storage; nothing was placed for it", c.name)));
                    Ok(mutation)
                })
            })
        }
        "set_output" => {
            let a: OutputArgs = args(name, arguments)?;
            let what = a.output.as_deref().map_or("Cleared the output of".to_string(), |o| format!("Set the output to {} on", quoted(o)));
            server.change(name, vec![plain(&a.rel_parts)], format!("{what} {}", path(&a.rel_parts)), search(&|root| mutations::set_output(root, &a.rel_parts, a.output.as_deref())))
        }
        "move" | "copy" => {
            let a: MoveArgs = args(name, arguments)?;
            let copy = name == "copy";
            let verb = if copy { "Copied" } else { "Moved" };
            let items = vec![format!("{} → {}", plain(&a.source), plain(&a.destination))];
            server.change(name, items, format!("{verb} {} into {}", path(&a.source), path(&a.destination)), |root| {
                indexed(root, |root| mutations::move_folder(root, &a.source, &a.destination, copy))
            })
        }
        "set_board" => {
            let a: BoardArgs = args(name, arguments)?;
            limit(name, "cards", a.cards.len(), MAX_BATCH)?;
            let items = a.cards.iter().map(|c| plain(&c.rel_parts)).collect();
            server.change(name, items, format!("Placed {} board cards", a.cards.len()), |root| edit::search_indexed(root, |root| mutations::set_board(root, &a.cards)))
        }
        "preview_project_groups" => {
            let a: GroupArgs = args(name, arguments)?;
            server.read(|_, root| Ok(json!({ "groups": mutations::preview_project_groups(root, &a.folder)? })))
        }
        "apply_project_groups" => {
            let a: GroupArgs = args(name, arguments)?;
            let plans = required(name, "plans", a.plans)?;
            limit(name, "plans", plans.len(), MAX_BATCH)?;
            limit(name, "files", plans.iter().map(|p| p.files.len()).sum(), MAX_FILES)?;
            let items = plans.iter().map(|p| format!("{} ({} files)", p.name, p.files.len())).collect();
            server.change(name, items, format!("Sorted {} projects in {}", plans.len(), path(&a.folder)), |root| {
                indexed(root, |root| mutations::apply_project_groups(root, &a.folder, &plans))
            })
        }
        "get_dictionary" => {
            let a: DictionaryArgs = args(name, arguments)?;
            server.read(|_, root| {
                let inherited: Vec<Spec> = (0..=a.rel_parts.len()).filter_map(|end| dictionary::read_specs(root, &a.rel_parts[..end]).transpose()).collect::<R<Vec<_>>>()?.into_iter().flatten().collect();
                Ok(json!({ "own": dictionary::read_specs(root, &a.rel_parts)?, "inherited": inherited }))
            })
        }
        "set_dictionary" => {
            let a: DictionaryArgs = args(name, arguments)?;
            let entries = required(name, "entries", a.entries)?;
            limit(name, "entries", entries.len(), MAX_ENTRIES)?;
            server.storage().and_then(|storage| {
                let root = storage.root()?;
                let (title, confirm) = asks(name, &[]);
                server.allow(&root, title, confirm, vec![format!("{} ({} entries)", plain(&a.rel_parts), entries.len())])?;
                let _guard = storage.guard();
                retrying(server.retry, || dictionary::write(&root, &a.rel_parts, &entries))?;
                let mut warnings = Vec::new();
                if let Err(e) = changes::record(&root, std::slice::from_ref(&a.rel_parts), false) {
                    warnings.push(format!("The open Folder Manager may show this change only after its next refresh: {e}"));
                }
                Ok(json!({ "changed": format!("Wrote the dictionary of {} with {} entries", quoted(&format!("/{}", plain(&a.rel_parts))), entries.len()), "warnings": warnings }))
            })
        }
        "undo_project_sort" => {
            args::<NoArgs>(name, arguments)?;
            server.change(name, vec!["The last project sort".to_string()], "Undid the last project sort".to_string(), |root| indexed(root, mutations::undo_project_sort))
        }
        _ => return node_call(server, name, arguments),
    })
}

fn node_call(server: &Server, name: &str, arguments: &Value) -> Result<R<Value>, RpcError> {
    if !TOOLS.iter().any(|t| t.0 == name) {
        return Err((INVALID_PARAMS, format!("Unknown tool: {name}")));
    }
    let a: NodeArgs = args(name, arguments)?;
    let rel = &a.rel_parts;
    let at = path(rel);
    let one = || vec![plain(rel)];
    Ok(match name {
        "rename" => {
            let new = needs(name, "name", a.name.as_deref())?;
            server.change(name, one(), format!("Renamed {at}"), |root| indexed(root, |root| mutations::rename_node(root, rel, new)))
        }
        "reorder" => {
            let dir = needs(name, "direction", a.direction)?;
            server.change(name, one(), format!("Moved {at} {}", if dir == MoveDirection::Up { "up" } else { "down" }), |root| {
                indexed(root, |root| mutations::move_node(root, rel, dir))
            })
        }
        "delete" => {
            let flags = (a.expect_ghost, a.confirm_non_empty);
            server.change(name, one(), format!("Sent {at} to the Recycle Bin"), |root| {
                edit::delete_node(root, rel, flags, &quiet, create_shortcuts, review_index::rebuild, server.recycle)
            })
        }
        "set_type" => {
            let base = needs(name, "type", a.node_type)?;
            server.change(name, one(), format!("Set the type of {at} to {}", base.as_str()), search(&|root| mutations::set_node_type(root, rel, base, a.status)))
        }
        "set_status" => {
            let status = needs(name, "status", a.status)?;
            server.change(name, one(), format!("Set the status of {at} to {}", status.as_str()), search(&|root| mutations::set_details(root, rel, Some(status), None)))
        }
        "set_owner" => {
            let owner = needs(name, "owner", a.owner.as_deref())?;
            server.change(name, one(), format!("Set the owner of {at}"), search(&|root| mutations::set_details(root, rel, None, Some(owner))))
        }
        "set_favorite" => {
            let on = needs(name, "on", a.on)?;
            server.change(name, one(), format!("Set favorite {on} on {at}"), search(&|root| mutations::set_favorite(root, rel, on)))
        }
        "set_important" => {
            let on = needs(name, "on", a.on)?;
            server.change(name, one(), format!("Set important {on} on {at}"), search(&|root| mutations::set_important(root, rel, on)))
        }
        "hide" => {
            let on = needs(name, "hidden", a.hidden)?;
            server.change(name, one(), format!("Set hidden {on} on {at}"), search(&|root| mutations::set_hidden(root, rel, on)))
        }
        "rename_file" => {
            let new = needs(name, "name", a.name.as_deref())?;
            server.change(name, one(), format!("Renamed the file {at}"), |root| mutations::rename_file(root, rel, new))
        }
        "delete_file" => server.change(name, one(), format!("Sent the file {at} to the Recycle Bin"), |root| mutations::delete_file(root, rel, server.recycle)),
        _ => return Err((INVALID_PARAMS, format!("Unknown tool: {name}"))),
    })
}
