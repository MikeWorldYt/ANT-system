// Decides, per tree node, what intake does with the files dropped directly inside its folder.
use serde_yaml::{Mapping, Value};

use crate::domain::tree_types::{child_entries, flags_in, type_base};
use crate::domain::config::{DATA_FLAG, REVIEW_PREFIX};
use crate::domain::names::{editable_part, support_kind};
use crate::services::tree::tree_node::is_reserved;
use crate::services::tree::tree_store::is_locked_type;

const SYSTEM_FILES: [&str; 2] = ["desktop.ini", "Thumbs.db"];
const ROOT_EXEMPT_MARKER: &str = "foldermanager";

pub(super) enum Rule<'a> {
    Root,
    // The folder and everything below it belong to the app or to a workboard, so intake never looks inside.
    Skip,
    Keep,
    Loose,
    Review(Option<&'a str>),
}

pub(super) fn rule<'a>(rel: &[String], node: &'a Mapping) -> Rule<'a> {
    if rel.is_empty() {
        return Rule::Root;
    }
    let raw = node_type(node);
    if is_reserved(rel) || is_locked_type(raw) || type_base(raw) == "WKB" {
        return Rule::Skip;
    }
    if rel.len() == 1 {
        // Storage's top-level folders are main nodes; send their files to their own review folder.
        return if type_base(raw) == "NODE" { Rule::Review(review_child(rel, node)) } else { Rule::Loose };
    }
    if type_base(raw) != "NODE" || flags_in(node).iter().any(|f| f == DATA_FLAG) {
        return Rule::Keep;
    }
    Rule::Review(review_child(rel, node))
}

pub(super) fn is_root_exempt_file(name: &str) -> bool {
    name.to_ascii_lowercase().contains(ROOT_EXEMPT_MARKER)
}

pub(super) fn is_skipped_file(name: &str) -> bool {
    name.starts_with('.') || SYSTEM_FILES.iter().any(|s| name.eq_ignore_ascii_case(s))
}

// An extra review stays beside the node's own until the user merges it, so the one named after the node goes first.
pub(crate) fn review_child<'a>(rel: &[String], node: &'a Mapping) -> Option<&'a str> {
    let reviews: Vec<&str> = child_entries(node).filter(|(name, child)| support_kind(name) == Some("REV") && type_base(node_type(child)) == "REV").map(|(name, _)| name).collect();
    let editable = rel.last().map_or("", |name| editable_part(name));
    let own = [format!("{REVIEW_PREFIX}{editable}"), format!("98.{editable}")];
    own.iter().find_map(|name| reviews.iter().find(|review| **review == name.as_str())).or(reviews.first()).copied()
}

pub(crate) fn node_type(node: &Mapping) -> &str {
    node.get("type").and_then(Value::as_str).unwrap_or("")
}
