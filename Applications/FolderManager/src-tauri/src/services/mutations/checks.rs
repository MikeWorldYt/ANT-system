// Refusals shared by the mutations: frozen nodes and names already taken in the target folder.
use std::fs;
use std::path::Path;

use serde_yaml::{Mapping, Value};

use crate::ant_core::tree_types::META_KEYS;
use crate::config::is_hidden_dir;
use crate::error::{FmError, R};
use crate::fs_safe::{io_err, same_name};
use crate::tree::tree_node::is_reserved;
use crate::tree::tree_store::is_locked_type;

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
    // A folder named like a meta key would overwrite its parent's type or status in the yaml.
    if META_KEYS.iter().any(|k| same_name(k, name)) || is_hidden_dir(name) {
        return Err(FmError::InvalidName { name: name.to_string(), reason: "reserved name".to_string() });
    }
    let clashes = |other: &str| same_name(other, name) && !ignore.contains(&other);
    let taken = FmError::AlreadyExists { name: name.to_string() };
    if parent_yaml.is_some_and(|m| m.keys().filter_map(Value::as_str).any(clashes)) {
        return Err(taken);
    }
    let entries = fs::read_dir(parent_abs).map_err(|e| io_err("list folder", parent_abs, &e))?;
    for entry in entries {
        let entry = entry.map_err(|e| io_err("list folder", parent_abs, &e))?;
        if clashes(&entry.file_name().to_string_lossy()) {
            return Err(taken);
        }
    }
    Ok(())
}
