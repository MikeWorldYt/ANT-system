// Which folders an internal drag may pick up, where it may put them down and how deep they may land.
use serde_yaml::{Mapping, Value};

use crate::ant_core::tree_types::{child_entries, flags_in, type_base};
use crate::config::DATA_FLAG;
use crate::error::{FmError, R};
use crate::intake::node_type;
use crate::mutations::Target;
use crate::tree::tree_node::is_reserved;
use crate::tree::tree_store::is_locked_type;

pub(super) fn refuse(rel: &[String], reason: impl Into<String>) -> FmError {
    FmError::InvalidPath { rel_parts: rel.to_vec(), reason: reason.into() }
}

// Everything under a reserved root belongs to the app, not only the root folder itself.
fn guard(rel: &[String], target: &Target, op: &str, open_review: bool) -> R<()> {
    if rel.first().is_some_and(|top| is_reserved(std::slice::from_ref(top))) {
        return Err(FmError::Reserved { rel_parts: rel.to_vec() });
    }
    if target.ghost {
        return Err(FmError::GhostNotAllowed { op: op.to_string() });
    }
    let review = open_review && type_base(&target.node_type) == "REV";
    if is_locked_type(&target.node_type) && !review {
        return Err(FmError::Locked { rel_parts: rel.to_vec(), node_type: target.node_type.clone() });
    }
    Ok(())
}

pub(super) fn check_source(rel: &[String], target: &Target, node: &Mapping) -> R<()> {
    guard(rel, target, "move", false)?;
    match type_base(&target.node_type) {
        "WKB" => Ok(()),
        "NODE" if flags_in(node).iter().any(|f| f == DATA_FLAG) => Ok(()),
        "NODE" => Err(refuse(rel, "only a data node or a workboard can be moved, and this node has no data flag")),
        "" => Err(refuse(rel, "the folder has no type in the tree yet; refresh and retry")),
        other => Err(refuse(rel, format!("{other} folders cannot be moved, only data nodes and workboards"))),
    }
}

pub(super) fn check_destination(rel: &[String], target: &Target) -> R<()> {
    // A 98.* review folder takes moved folders as it takes dropped files; the other locked types do not.
    guard(rel, target, "move into", true)?;
    match type_base(&target.node_type) {
        "NODE" | "WKB" | "REV" => Ok(()),
        "" => Err(refuse(rel, "the folder has no type in the tree yet; refresh and retry")),
        other => Err(refuse(rel, format!("{other} folders do not take moved folders"))),
    }
}

pub(super) fn check_not_inside(source: &[String], destination: &[String]) -> R<()> {
    if destination.starts_with(source) {
        return Err(refuse(destination, "a folder cannot be moved into itself or into one of its own folders"));
    }
    Ok(())
}

// A move past the limit is allowed; the caller only warns that the deeper folders leave the tree.
pub(super) fn depth_warning(name: &str, node: &Mapping, destination: &[String], limit: usize, copy: bool) -> Option<String> {
    let verb = if copy { "copied" } else { "moved" };
    (destination.len() + 1 + height(node) > limit)
        .then(|| format!("{name} {verb}; folders below level {limit} are not shown in the tree (they stay on disk)"))
}

// Locked folders on the last level live on disk only, as add_node leaves them, so they never count toward the depth.
fn height(node: &Mapping) -> usize {
    child_entries(node).filter(|(_, child)| !is_locked_type(node_type(child))).map(|(_, child)| 1 + height(child)).max().unwrap_or(0)
}

// Entries that land past the depth limit stay on disk only, the same shape a sync gives them.
pub(super) fn prune(node: &mut Mapping, level: usize, limit: usize) {
    let names: Vec<String> = child_entries(node).map(|(name, _)| name.to_string()).collect();
    for name in names {
        if level >= limit {
            node.remove(name.as_str());
        } else if let Some(Value::Mapping(child)) = node.get_mut(name.as_str()) {
            prune(child, level + 1, limit);
        }
    }
}
