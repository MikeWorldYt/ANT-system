// JSON schemas of the read tools: what each takes and the shape of what it returns.
use serde_json::{json, Value};

fn strings() -> Value {
    json!({ "type": "array", "items": { "type": "string" } })
}

fn nullable(kind: &str) -> Value {
    json!({ "type": [kind, "null"] })
}

fn rel_parts(min: usize) -> Value {
    json!({ "type": "array", "items": { "type": "string" }, "minItems": min,
        "description": "Folder names from the storage root down, e.g. [\"100.WORK\", \"01.Project\"]" })
}

fn object(properties: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": properties, "required": required, "additionalProperties": false })
}

// Output objects may gain fields later, so they stay open where inputs are closed.
fn output(properties: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": properties, "required": required })
}

fn brief() -> Value {
    let flag = json!({ "type": "boolean" });
    output(
        json!({
            "name": { "type": "string" }, "relParts": strings(), "type": nullable("string"), "status": nullable("string"),
            "flags": strings(), "file": flag, "ghost": flag, "linked": flag, "locked": flag, "reserved": flag,
            "favorite": flag, "important": flag, "hidden": flag, "hasChildren": flag,
            "children": { "type": "array", "items": { "type": "object" } },
        }),
        &["name", "relParts"],
    )
}

fn nodes() -> Value {
    json!({ "type": "array", "items": brief() })
}

pub(super) fn get_storage() -> (Value, Value) {
    let output = output(
        json!({
            "root": { "type": "string" }, "appVersion": { "type": "string" }, "maxDepth": { "type": "integer" },
            "depthOptions": { "type": "array", "items": { "type": "integer" } }, "depthWarning": nullable("string"),
        }),
        &["root", "appVersion", "maxDepth"],
    );
    (object(json!({}), &[]), output)
}

pub(super) fn get_tree() -> (Value, Value) {
    let input = object(
        json!({ "depth": { "type": "integer", "minimum": 1, "description": "Levels to return; at most the storage's depth limit, which is the default" } }),
        &[],
    );
    let output = output(
        json!({ "root": { "type": "string" }, "depth": { "type": "integer" }, "maxDepth": { "type": "integer" }, "nodes": nodes() }),
        &["root", "depth", "maxDepth", "nodes"],
    );
    (input, output)
}

pub(super) fn list_children() -> (Value, Value) {
    (object(json!({ "relParts": rel_parts(0) }), &["relParts"]), output(json!({ "nodes": nodes() }), &["nodes"]))
}

pub(super) fn get_node() -> (Value, Value) {
    let output = output(
        json!({
            "name": { "type": "string" }, "relParts": strings(), "depth": { "type": "integer" },
            "nodeType": { "type": ["object", "null"], "properties": { "raw": { "type": "string" }, "base": { "type": "string" }, "level": nullable("integer") } },
            "status": nullable("string"), "statusRaw": nullable("string"), "owner": nullable("string"), "flags": strings(),
            "info": { "type": ["object", "null"], "properties": {
                "fields": { "type": "array", "items": output(json!({ "key": { "type": "string" }, "value": { "type": "string" } }), &["key", "value"]) },
                "error": nullable("string"),
            } },
            "absPath": { "type": "string" }, "actions": { "type": "object" },
        }),
        &["name", "relParts", "flags"],
    );
    (object(json!({ "relParts": rel_parts(1) }), &["relParts"]), output)
}

const QUERY: &str = "Words to match plus optional filters: `in:200.WORK` limits to a folder, `key:value` matches a field (type, status, owner, any info key)";

pub(super) fn search_folders() -> (Value, Value) {
    let input = object(
        json!({
            "query": { "type": "string", "description": QUERY },
            "disk": { "type": "boolean", "description": "Walk the folders on disk instead of the index; slower, finds folders the index does not hold" },
        }),
        &["query"],
    );
    let paths = json!({ "type": "array", "items": strings() });
    (input, output(json!({ "paths": paths }), &["paths"]))
}

pub(super) fn search_files() -> (Value, Value) {
    let hit = output(json!({ "folder": strings(), "name": { "type": "string" } }), &["folder", "name"]);
    let input = object(json!({ "query": { "type": "string", "description": QUERY } }), &["query"]);
    (input, output(json!({ "hits": { "type": "array", "items": hit } }), &["hits"]))
}

pub(super) fn suggest_search() -> (Value, Value) {
    let input = object(
        json!({
            "key": { "type": "string", "description": "The filter being typed, e.g. `in`, `status` or `owner`" },
            "prefix": { "type": "string", "description": "What is typed after the colon so far" },
        }),
        &["key", "prefix"],
    );
    let item = output(json!({ "value": { "type": "string" }, "label": { "type": "string" }, "hint": { "type": "string" } }), &["value", "label", "hint"]);
    (input, output(json!({ "suggestions": { "type": "array", "items": item } }), &["suggestions"]))
}
