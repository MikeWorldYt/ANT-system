// JSON schemas of the write tools: what each takes, and the short note of what changed that most of them return.
use serde_json::{json, Value};

use super::schema::{nullable, object, output, rel_parts, strings};

const STATUSES: [&str; 4] = ["in-progress", "for-review", "hold", "done"];

fn text(description: &str) -> Value {
    json!({ "type": "string", "minLength": 1, "description": description })
}

fn flag(description: &str) -> Value {
    json!({ "type": "boolean", "description": description })
}

fn status() -> Value {
    json!({ "type": "string", "enum": STATUSES })
}

fn change() -> Value {
    output(
        json!({
            "changed": { "type": "string" },
            "relParts": { "type": ["array", "null"], "items": { "type": "string" }, "description": "Where the changed node is now, when it still exists" },
            "levels": { "type": "array", "items": strings(), "description": "Folders whose children changed; read them again with list_children" },
            "fullTree": { "type": "boolean", "description": "The change reached beyond its folders; read get_tree again" },
            "warnings": strings(),
        }),
        &["changed", "levels", "fullTree", "warnings"],
    )
}

fn on_node(properties: Value, required: &[&str]) -> (Value, Value) {
    let mut properties = properties;
    properties["relParts"] = rel_parts(1);
    let mut all = vec!["relParts"];
    all.extend_from_slice(required);
    (object(properties, &all), change())
}

const NAME: &str = "The editable part of the name; the app keeps the id or track number in front and cleans it as a rename does";

pub(super) fn add_node() -> (Value, Value) {
    let input = object(json!({ "parent": rel_parts(0), "name": text(NAME) }), &["parent", "name"]);
    (input, change())
}

pub(super) fn add_workboard() -> (Value, Value) {
    let input = object(json!({ "parent": rel_parts(1), "name": text(NAME) }), &["parent", "name"]);
    (input, change())
}

pub(super) fn rename() -> (Value, Value) {
    on_node(json!({ "name": text(NAME) }), &["name"])
}

pub(super) fn move_folder() -> (Value, Value) {
    let input = object(json!({ "source": rel_parts(1), "destination": rel_parts(1) }), &["source", "destination"]);
    (input, change())
}

pub(super) fn reorder() -> (Value, Value) {
    on_node(json!({ "direction": { "type": "string", "enum": ["up", "down"] } }), &["direction"])
}

pub(super) fn delete() -> (Value, Value) {
    on_node(
        json!({
            "confirmNonEmpty": flag("Required to delete a folder that holds files or folders; without it the call is refused with their count"),
            "expectGhost": flag("True to forget a folder that is missing on disk; it must match what get_node reports as ghost"),
        }),
        &[],
    )
}

pub(super) fn set_type() -> (Value, Value) {
    on_node(json!({ "type": { "type": "string", "enum": ["NODE", "WKB"] }, "status": status() }), &["type"])
}

pub(super) fn set_status() -> (Value, Value) {
    on_node(json!({ "status": status() }), &["status"])
}

pub(super) fn set_owner() -> (Value, Value) {
    on_node(json!({ "owner": { "type": "string", "minLength": 1, "maxLength": 100 } }), &["owner"])
}

pub(super) fn set_flag() -> (Value, Value) {
    on_node(json!({ "on": { "type": "boolean" } }), &["on"])
}

pub(super) fn hide() -> (Value, Value) {
    on_node(json!({ "hidden": { "type": "boolean" } }), &["hidden"])
}

pub(super) fn set_board() -> (Value, Value) {
    let card = object(
        json!({ "relParts": rel_parts(1), "column": { "type": "string", "enum": ["backlog", "todo", "progress", "done"] }, "order": { "type": "integer", "minimum": 0 } }),
        &["relParts", "column", "order"],
    );
    (object(json!({ "cards": { "type": "array", "items": card, "minItems": 1 } }), &["cards"]), change())
}

pub(super) fn rename_file() -> (Value, Value) {
    on_node(json!({ "name": text("The file's whole new name, extension included") }), &["name"])
}

pub(super) fn delete_file() -> (Value, Value) {
    on_node(json!({}), &[])
}

pub(super) fn preview_project_groups() -> (Value, Value) {
    let group = output(
        json!({
            "name": { "type": "string" }, "files": strings(), "tier": { "type": "string", "enum": ["sure", "likely"] },
            "owner": nullable("string"), "ownerConflict": { "type": "boolean" }, "created": nullable("string"),
        }),
        &["name", "files", "tier"],
    );
    (object(json!({ "folder": rel_parts(1) }), &["folder"]), output(json!({ "groups": { "type": "array", "items": group } }), &["groups"]))
}

pub(super) fn apply_project_groups() -> (Value, Value) {
    let plan = object(
        json!({ "name": { "type": "string", "minLength": 1 }, "files": strings(), "owner": nullable("string"), "created": nullable("string") }),
        &["name", "files"],
    );
    (object(json!({ "folder": rel_parts(1), "plans": { "type": "array", "items": plan, "minItems": 1 } }), &["folder", "plans"]), change())
}

pub(super) fn undo_project_sort() -> (Value, Value) {
    (object(json!({}), &[]), change())
}

fn entry() -> Value {
    let words = |description: &str| json!({ "type": "array", "items": { "type": "string", "minLength": 1 }, "description": description });
    object(
        json!({
            "folder": text("The subfolder its files group into, also the entry's heading; \"Do not group\" keeps them in place; a name starting with a node id (\"07.Name\") sends them to that node's review folder"),
            "term": words("Names of the subject; matched as whole words ignoring case and accents, and removed from the file name"),
            "topics": words("Related names, such as characters; matched like terms, kept once in the name and their repeats removed"),
            "only": words("Exact words, case and symbols included; kept in the name"),
            "exclude": words("Words removed from the name of a grouped file; they never decide the group"),
            "ext": words("Extensions such as \".jpg\""),
            "pattern": words("Case-insensitive regular expressions on the name without extension; the match is removed"),
        }),
        &["folder"],
    )
}

pub(super) fn get_dictionary() -> (Value, Value) {
    let list = json!({ "type": "array", "items": entry() });
    let own = json!({ "type": ["array", "null"], "items": entry(), "description": "This folder's own entries; null when it has no dictionary.md" });
    (object(json!({ "relParts": rel_parts(0) }), &["relParts"]), output(json!({ "own": own, "inherited": list }), &["own", "inherited"]))
}

pub(super) fn set_dictionary() -> (Value, Value) {
    let input = object(json!({ "relParts": rel_parts(0), "entries": { "type": "array", "items": entry() } }), &["relParts", "entries"]);
    (input, output(json!({ "changed": { "type": "string" }, "warnings": strings() }), &["changed", "warnings"]))
}
