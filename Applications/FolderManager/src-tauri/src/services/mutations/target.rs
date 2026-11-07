// Resolves a node path to its folder on disk, telling real folders from tree entries missing on disk.
use std::fs;
use std::path::{Path, PathBuf};

use serde_yaml::Mapping;

use crate::domain::names::is_workboard_name;
use crate::domain::tree_types::META_KEYS;
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::{io_err, resolve};
use crate::services::tree::tree_store::{get_node, get_type};

pub(crate) struct Target {
    pub abs: PathBuf,
    pub ghost: bool,
    pub node_type: String,
}

pub(crate) fn parent_of(rel: &[String]) -> &[String] {
    rel.split_last().map_or(&[], |(_, parent)| parent)
}

pub(crate) fn split(rel: &[String]) -> R<(&String, &[String])> {
    rel.split_last().ok_or_else(|| FmError::InvalidPath {
        rel_parts: Vec::new(),
        reason: "the storage root is not a node".to_string(),
    })
}

pub(crate) fn child_rel(parent: &[String], name: &str) -> Vec<String> {
    let mut rel = parent.to_vec();
    rel.push(name.to_string());
    rel
}

pub(crate) fn locate(root: &Path, data: &Mapping, rel: &[String]) -> R<Target> {
    split(rel)?;
    // A folder named like a meta key would make its parent's type or status look like a node.
    if let Some(meta) = rel.iter().find(|p| META_KEYS.contains(&p.as_str())) {
        return Err(FmError::InvalidName { name: meta.clone(), reason: "reserved name".to_string() });
    }
    let abs = resolve(root, rel)?;
    let on_disk = exactly_on_disk(root, rel)?;
    if !on_disk && get_node(data, rel).is_none() {
        return Err(FmError::NotFound { rel_parts: rel.to_vec() });
    }
    // A workboard-named folder past the depth limit has no tree entry but edits like any workboard.
    let node_type = match get_node(data, rel) {
        None if on_disk && rel.last().is_some_and(|name| is_workboard_name(name)) => "WKB".to_string(),
        _ => get_type(data, rel),
    };
    Ok(Target { abs, ghost: !on_disk, node_type })
}

pub(super) fn exactly_on_disk(root: &Path, rel: &[String]) -> R<bool> {
    let mut dir = root.to_path_buf();
    for part in rel {
        if !has_exact_dir(&dir, part)? {
            return Ok(false);
        }
        dir.push(part);
    }
    Ok(true)
}

// Exact match, because Windows would also open "01.math" for a tree key "01.Math".
fn has_exact_dir(dir: &Path, name: &str) -> R<bool> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(io_err("list folder", dir, &e)),
    };
    for entry in entries {
        let entry = entry.map_err(|e| io_err("list folder", dir, &e))?;
        if entry.file_name() == name {
            let kind = entry.file_type().map_err(|e| io_err("inspect path", &entry.path(), &e))?;
            return Ok(kind.is_dir());
        }
    }
    Ok(false)
}
