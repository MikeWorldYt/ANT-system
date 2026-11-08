// JSON schemas of the interface tools: what each takes, and the window state every one of them answers with.
use serde_json::{json, Value};

use super::schema::{nullable, object, output, rel_parts, strings};

pub(super) const VIEWS: [&str; 5] = ["cards", "tree", "columns", "grid", "list"];

fn path_or_null(description: &str) -> Value {
    json!({ "type": ["array", "null"], "items": { "type": "string" }, "description": description })
}

fn state() -> Value {
    let tab = output(
        json!({
            "label": { "type": "string" }, "relParts": strings(), "virtual": nullable("string"),
            "view": { "type": "string" }, "active": { "type": "boolean" },
        }),
        &["label", "relParts", "view", "active"],
    );
    let split = json!({ "type": ["object", "null"], "properties": {
        "focused": { "type": "string", "enum": ["left", "right"] },
        "otherRelParts": path_or_null("What the pane without focus has selected"),
        "otherView": { "type": "string" },
    } });
    output(
        json!({
            "root": nullable("string"), "tabs": { "type": "array", "items": tab }, "activeTab": { "type": "integer" },
            "split": split, "view": { "type": "string", "enum": VIEWS },
            "virtual": { "type": ["string", "null"], "description": "The open virtual list, such as favorites or the board, when one is open" },
            "selection": path_or_null("The selected node of the active pane"),
            "selected": { "type": "array", "items": strings(), "description": "Every selected node, when several are" },
        }),
        &["root", "tabs", "activeTab", "split", "view", "virtual", "selection", "selected"],
    )
}

pub(super) fn get_state() -> (Value, Value) {
    (object(json!({}), &[]), state())
}

pub(super) fn reveal() -> (Value, Value) {
    (object(json!({ "relParts": rel_parts(1) }), &["relParts"]), state())
}

pub(super) fn place() -> (Value, Value) {
    let input = json!({ "type": ["array", "null"], "items": { "type": "string" },
        "description": "The folder to open, as folder names from the storage root down; leave it out for the storage root" });
    (object(json!({ "relParts": input }), &[]), state())
}

pub(super) fn set_view() -> (Value, Value) {
    (object(json!({ "view": { "type": "string", "enum": VIEWS } }), &["view"]), state())
}
