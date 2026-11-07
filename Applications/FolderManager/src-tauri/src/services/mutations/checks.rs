// Refusals shared by the mutations: frozen nodes and names already taken in the target folder.
use std::path::Path;

use serde_yaml::{Mapping, Value};

use crate::domain::tree_types::META_KEYS;
use super::target::{locate, Target};
use crate::domain::config::is_hidden_dir;
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::{dir_entries, same_name};
use crate::services::tree::tree_node::is_reserved;
use crate::services::tree::tree_store::is_locked_type;

// Most edits need a real folder that is neither reserved nor locked; a ghost exists only in the tree.
pub(crate) fn editable_target(root: &Path, data: &Mapping, rel: &[String], op: &str) -> R<Target> {
    let target = locate(root, data, rel)?;
    guard_frozen(rel, &target.node_type)?;
    if target.ghost {
        return Err(FmError::GhostNotAllowed { op: op.to_string() });
    }
    Ok(target)
}

pub(crate) fn guard_frozen(rel: &[String], node_type: &str) -> R<()> {
    if is_reserved(rel) {
        return Err(FmError::Reserved { rel_parts: rel.to_vec() });
    }
    if is_locked_type(node_type) {
        return Err(FmError::Locked { rel_parts: rel.to_vec(), node_type: node_type.to_string() });
    }
    Ok(())
}

pub(crate) fn ensure_free(
    parent_abs: &Path,
    parent_yaml: Option<&Mapping>,
    name: &str,
    ignore: &[&str],
) -> R<()> {
    // A folder named like a meta key would overwrite its parent's type or status in the tree.
    if META_KEYS.iter().any(|k| same_name(k, name)) || is_hidden_dir(name) {
        return Err(FmError::InvalidName { name: name.to_string(), reason: "reserved name".to_string() });
    }
    let clashes = |other: &str| same_name(other, name) && !ignore.contains(&other);
    let taken = FmError::AlreadyExists { name: name.to_string() };
    if parent_yaml.is_some_and(|m| m.keys().filter_map(Value::as_str).any(clashes)) {
        return Err(taken);
    }
    for entry in dir_entries(parent_abs, "list folder")? {
        let entry = entry?;
        if clashes(&entry.file_name().to_string_lossy()) {
            return Err(taken);
        }
    }
    Ok(())
}
