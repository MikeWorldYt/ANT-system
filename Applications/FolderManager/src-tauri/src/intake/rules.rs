// Decides, per tree node, what intake does with the files dropped directly inside its folder.
use serde_yaml::{Mapping, Value};

use crate::ant_core::tree_types::{child_entries, flags_in, type_base};
use crate::config::DATA_FLAG;
use crate::tree::tree_node::is_reserved;
use crate::tree::tree_store::is_locked_type;

const REVIEW_PREFIX: &str = "98.";
const EXEMPT_MARK: &str = "FolderManager";
const SYSTEM_FILES: [&str; 2] = ["desktop.ini", "Thumbs.db"];

pub(super) enum Rule<'a> {
    // The folder and everything below it belong to the app or to a workboard, so intake never looks inside.
    Skip,
    Keep,
    Loose,
    Review(Option<&'a str>),
}

pub(super) fn rule<'a>(rel: &[String], node: &'a Mapping) -> Rule<'a> {
    if rel.is_empty() {
        return Rule::Keep;
    }
    let raw = node_type(node);
    if is_reserved(rel) || is_locked_type(raw) || type_base(raw) == "WKB" {
        return Rule::Skip;
    }
    if rel.len() == 1 {
        return Rule::Loose;
    }
    if type_base(raw) != "NODE" || flags_in(node).iter().any(|f| f == DATA_FLAG) {
        return Rule::Keep;
    }
    Rule::Review(review_child(node))
}

pub(super) fn is_skipped_file(name: &str) -> bool {
    name.starts_with('.') || is_exempt(name) || SYSTEM_FILES.iter().any(|s| name.eq_ignore_ascii_case(s))
}

pub(super) fn is_exempt(name: &str) -> bool {
    name.contains(EXEMPT_MARK)
}

fn review_child(node: &Mapping) -> Option<&str> {
    child_entries(node)
        .find(|(name, child)| name.starts_with(REVIEW_PREFIX) && type_base(node_type(child)) == "REV")
        .map(|(name, _)| name)
}

fn node_type(node: &Mapping) -> &str {
    node.get("type").and_then(Value::as_str).unwrap_or("")
}
