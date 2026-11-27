// The automation tools: turn a folder's automations on, plan a run as a dry run the user reads, then apply exactly the rows they saw.
use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use super::gate::MAX_FILES;
use super::protocol::{RpcError, INVALID_PARAMS};
use super::retry::retrying;
use super::schema::{object, output, rel_parts, strings};
use super::tools::args;
use super::Server;
use crate::domain::config::AUTOM_FLAG;
use crate::domain::error::R;
use crate::infra::storage::changes;
use crate::services::automations::{self, chain::ApplyOutcome, PlannedFile, Picks};
use crate::services::downloads;
use crate::services::{dictionary_ai, edit, mutations};

type Schemas = fn() -> (Value, Value);

const AUTOMATIONS: [&str; 6] = ["reduce", "sort", "group", "addDate", "removeDate", "removeSymbols"];

const TOOLS: [(&str, &str, &str, Schemas); 5] = [
    ("set_automations", "Turn automations on", "Lets a folder run automations (its `autom` flag); review, screenshot and recording folders always can. Read ant://guide/automations first.", set_automations),
    ("set_review_automation", "Set automatic run", "Makes one automation run on its own on every file that lands in a folder that runs automations.", set_review_automation),
    ("plan_automations", "Plan automations", "Dry run: the new name, project and destination each file of a folder would get; changes nothing. Show it to the user before apply_automations.", plan_automations),
    ("apply_automations", "Run automations", "Renames and moves files as the rows of a plan_automations dry run say; the user confirms in the window.", apply_automations),
    ("build_dictionary", "Build dictionary", "Has the connected AI client (Claude Code, Codex or OpenCode) write a folder's `.ant/dictionary.md` from the words its review folder's file names share, keeping the entries already there, and replaces the file whole; the user confirms in the window. Takes minutes and needs one of them connected and signed in. Check the result with get_dictionary and plan_automations.", build_dictionary),
];

pub(super) fn has(name: &str) -> bool {
    TOOLS.iter().any(|t| t.0 == name)
}

pub(super) fn list() -> Vec<Value> {
    TOOLS
        .iter()
        .map(|(name, title, description, schemas)| {
            let (input, out) = schemas();
            let annotations = match *name {
                "plan_automations" => json!({ "title": title, "readOnlyHint": true, "openWorldHint": false }),
                "apply_automations" => json!({ "title": title, "readOnlyHint": false, "destructiveHint": true, "idempotentHint": false, "openWorldHint": false }),
                "build_dictionary" => json!({ "title": title, "readOnlyHint": false, "destructiveHint": true, "idempotentHint": false, "openWorldHint": true }),
                _ => json!({ "title": title, "readOnlyHint": false, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false }),
            };
            json!({ "name": name, "title": title, "description": description, "inputSchema": input, "outputSchema": out, "annotations": annotations })
        })
        .collect()
}

fn change() -> Value {
    output(json!({ "changed": { "type": "string" }, "levels": { "type": "array", "items": strings() }, "fullTree": { "type": "boolean" }, "warnings": strings() }), &["changed", "warnings"])
}

fn folder() -> Value {
    json!({ "type": "array", "items": { "type": "string" }, "description": "The folder as relParts; the Downloads folder is [\"::Downloads\"]" })
}

fn set_automations() -> (Value, Value) {
    (object(json!({ "relParts": rel_parts(2), "on": { "type": "boolean" } }), &["relParts", "on"]), change())
}

fn set_review_automation() -> (Value, Value) {
    let input = object(json!({ "relParts": rel_parts(1), "automation": { "type": "string", "enum": AUTOMATIONS }, "on": { "type": "boolean" } }), &["relParts", "automation", "on"]);
    (input, change())
}

fn picks() -> Value {
    let flag = |d: &str| json!({ "type": "boolean", "description": d });
    json!({
        "folder": folder(),
        "offset": { "type": "integer", "minimum": 0, "description": "Rows to skip; pass the previous result's `next` to read the next page" },
        "only": { "type": "array", "items": { "type": "string" }, "description": "File names to plan; empty plans every file in the folder" },
        "reduce": flag("Shorten names with the dictionary's terms"),
        "sort": flag("Sort files into projects (worknodes)"),
        "group": flag("Move files into the dictionary's subfolders"),
        "addDate": flag("Put the file's date in front of its name"),
        "removeDate": flag("Take a date out of the name; wins over addDate"),
        "removeSymbols": flag("Take brackets and stray symbols out of the name, keeping a \" (N)\" copy counter"),
    })
}

fn row() -> Value {
    json!({
        "type": "object",
        "properties": {
            "file": { "type": "string" }, "name": { "type": "string" }, "project": { "type": ["string", "null"] }, "sure": { "type": "boolean" },
            "folder": { "type": ["string", "null"] }, "target": { "type": ["array", "null"], "items": { "type": "string" } }, "create": { "type": "boolean" },
        },
        "required": ["file", "name"],
    })
}

fn plan_automations() -> (Value, Value) {
    (object(picks(), &["folder"]), output(json!({ "rows": { "type": "array", "items": row() }, "total": { "type": "integer" }, "next": { "type": ["integer", "null"], "description": "Offset of the next page; null on the last" } }), &["rows", "total", "next"]))
}

fn apply_automations() -> (Value, Value) {
    let input = object(json!({ "folder": folder(), "rows": { "type": "array", "minItems": 1, "items": row(), "description": "The rows of plan_automations as shown to the user; drop or edit rows the user changed" } }), &["folder", "rows"]);
    (input, change())
}

fn build_dictionary() -> (Value, Value) {
    (object(json!({ "folder": folder() }), &["folder"]), output(json!({ "changed": { "type": "string" }, "warnings": strings() }), &["changed", "warnings"]))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FlagArgs {
    rel_parts: Vec<String>,
    automation: Option<String>,
    on: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlanArgs {
    folder: Vec<String>,
    #[serde(default)]
    offset: usize,
    #[serde(default)]
    only: Vec<String>,
    #[serde(default)]
    reduce: bool,
    #[serde(default)]
    sort: bool,
    #[serde(default)]
    group: bool,
    #[serde(default)]
    add_date: bool,
    #[serde(default)]
    remove_date: bool,
    #[serde(default)]
    remove_symbols: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BuildArgs {
    folder: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApplyArgs {
    folder: Vec<String>,
    rows: Vec<PlannedFile>,
}

fn in_downloads(folder: &[String]) -> Option<&[String]> {
    folder.split_first().filter(|(first, _)| *first == downloads::ROOT).map(|(_, rest)| rest)
}

fn plan(root: &Path, a: &PlanArgs) -> R<Vec<PlannedFile>> {
    let picks = Picks { reduce: a.reduce, sort: a.sort, group: a.group, add_date: a.add_date, remove_date: a.remove_date, remove_symbols: a.remove_symbols };
    match in_downloads(&a.folder) {
        Some(sub) => automations::downloads::plan(root, &downloads::system_base()?, sub, &a.only, picks),
        None => automations::plan(root, &a.folder, &a.only, picks),
    }
}

fn apply(root: &Path, a: &ApplyArgs) -> R<ApplyOutcome> {
    match in_downloads(&a.folder) {
        Some(sub) => automations::downloads::apply(root, &downloads::system_base()?, sub, &a.rows),
        None => automations::apply(root, &a.folder, &a.rows).map(|changed| ApplyOutcome { changed, chained: Vec::new() }),
    }
}

// The same build as the editor's button; with no editor to review it in, the answer is written as the editor would save it.
fn build(root: &Path, folder: &[String]) -> R<String> {
    let work = dictionary_ai::work_dir(folder, in_downloads(folder).is_some());
    match in_downloads(folder) {
        Some(sub) => dictionary_ai::build(&downloads::path_of(&downloads::system_base()?, sub)?, &work, "Downloads", &automations::downloads::read_text()?),
        None => {
            let current = automations::dictionary::read_text(root, folder)?;
            dictionary_ai::build(&dictionary_ai::review_dir(root, folder)?, &work, folder.last().map_or("the storage", String::as_str), &current)
        }
    }
}

fn save(root: &Path, folder: &[String], text: &str) -> R<()> {
    match in_downloads(folder) {
        Some(_) => automations::downloads::write_text(text),
        None => automations::dictionary::write_text(root, folder, text),
    }
}

pub(super) fn call(server: &Server, name: &str, arguments: &Value) -> Result<R<Value>, RpcError> {
    Ok(match name {
        "set_automations" | "set_review_automation" => {
            let a: FlagArgs = args(name, arguments)?;
            let at = a.rel_parts.join("/");
            match (name, a.automation.as_deref()) {
                ("set_automations", None) => server.change_as("Turn automations on", false, vec![at.clone()], format!("Set automations {} on {at:?}", a.on), |root| {
                    edit::search_indexed(root, |root| mutations::set_user_flag(root, &a.rel_parts, AUTOM_FLAG, a.on))
                }),
                ("set_review_automation", Some(which)) if AUTOMATIONS.contains(&which) => server.change_as("Set automatic run", false, vec![at.clone()], format!("Set automatic {which} {} on {at:?}", a.on), |root| {
                    mutations::set_review_automation(root, &a.rel_parts, which, a.on)
                }),
                _ => return Err((INVALID_PARAMS, format!("Invalid arguments for {name}: `automation` must be one of {AUTOMATIONS:?} and only set_review_automation takes it"))),
            }
        }
        "plan_automations" => {
            let a: PlanArgs = args(name, arguments)?;
            server.read(|_, root| {
                let rows = plan(root, &a)?;
                let total = rows.len();
                let end = total.min(a.offset.saturating_add(MAX_FILES));
                let next = (end < total).then_some(end);
                Ok(json!({ "rows": rows.get(a.offset.min(total)..end).unwrap_or_default(), "total": total, "next": next }))
            })
        }
        "apply_automations" => {
            let a: ApplyArgs = args(name, arguments)?;
            if a.rows.len() > MAX_FILES {
                return Err((INVALID_PARAMS, format!("Invalid arguments for {name}: {} rows in one call, at most {MAX_FILES}; nothing was changed", a.rows.len())));
            }
            let items = a.rows.iter().map(|r| format!("{} → {}", r.file, r.name)).collect();
            server.storage().and_then(|storage| {
                let root = storage.root()?;
                server.allow(&root, "Run automations", true, items)?;
                let _guard = storage.guard();
                let outcome = retrying(server.retry, || apply(&root, &a))?;
                let mut warnings: Vec<String> = outcome.chained.iter().map(|c| format!("{} also ran its review folder's automations", c.name)).collect();
                if let Err(e) = changes::record(&root, &[], true) {
                    warnings.push(format!("The open Folder Manager may show this change only after its next refresh: {e}"));
                }
                Ok(json!({ "changed": format!("Ran the automations on {} files in {:?}", outcome.changed, a.folder.join("/")), "levels": [], "fullTree": true, "warnings": warnings }))
            })
        }
        "build_dictionary" => {
            let a: BuildArgs = args(name, arguments)?;
            server.storage().and_then(|storage| {
                let root = storage.root()?;
                server.allow(&root, "Build dictionary", true, vec![a.folder.join("/")])?;
                // Built before the lock is taken, since the AI client may work for minutes.
                let text = build(&root, &a.folder)?;
                let _guard = storage.guard();
                retrying(server.retry, || save(&root, &a.folder, &text))?;
                let mut warnings = Vec::new();
                if in_downloads(&a.folder).is_none() {
                    if let Err(e) = changes::record(&root, std::slice::from_ref(&a.folder), false) {
                        warnings.push(format!("The open Folder Manager may show this change only after its next refresh: {e}"));
                    }
                }
                Ok(json!({ "changed": format!("Built the dictionary of {:?} with AI", a.folder.join("/")), "warnings": warnings }))
            })
        }
        _ => return Err((INVALID_PARAMS, format!("Unknown tool: {name}"))),
    })
}
