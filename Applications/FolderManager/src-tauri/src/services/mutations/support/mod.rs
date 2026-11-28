// A node's support folders added or quit one at a time, with the tree entry kept in step; only an empty one can be quit.
#[cfg(test)]
mod tests;

use std::path::Path;

use serde::Serialize;
use serde_yaml::Mapping;

use super::recycle::move_to_bin;
use super::{child_rel, ensure_free, finish, guard_frozen, load_synced_at, locate, persist, scope, subtrees, Mutation, Target};
use crate::domain::config::DATA_DIR;
use crate::domain::error::{FmError, R};
use crate::domain::names::support_kind;
use crate::domain::support_rules::{allowed_kinds, support_folder_name};
use crate::domain::tree_types::type_base;
use crate::infra::fs_safe::{dir_entries, validate_component, Journal, StorageLock};
use crate::infra::scan::list_subdirs;
use crate::services::tree::depth;
use crate::services::tree::tree_node::is_reserved;
use crate::services::tree::tree_store::{delete_key, get_node, get_node_mut, get_type};

// `empty` only matters for a folder that exists: the window disables Quit on one that holds anything.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SupportRow {
    pub kind: &'static str,
    pub name: String,
    pub exists: bool,
    pub empty: bool,
}

// Empty for anything that is not a real node able to hold support folders: ghosts, worknodes, support folders and the app's own folders.
fn kinds_of(root: &Path, data: &Mapping, rel: &[String], target: &Target) -> R<Vec<&'static str>> {
    let Some(name) = rel.last().filter(|_| !is_reserved(rel) && !target.ghost) else { return Ok(Vec::new()) };
    if (1..rel.len()).any(|n| type_base(&get_type(data, &rel[..n])) != "NODE") {
        return Ok(Vec::new());
    }
    Ok(allowed_kinds(name, &target.node_type, rel.len(), depth::limit(root)?))
}

fn editable_holder(root: &Path, data: &Mapping, rel: &[String]) -> R<(Target, Vec<&'static str>)> {
    let target = locate(root, data, rel)?;
    guard_frozen(rel, &target.node_type)?;
    let kinds = kinds_of(root, data, rel, &target)?;
    if kinds.is_empty() {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "this folder cannot hold support folders".to_string() });
    }
    Ok((target, kinds))
}

// The .ant folder is the app's own marker, so it does not count as content.
fn is_empty(dir: &Path) -> R<bool> {
    for entry in dir_entries(dir, "list folder")? {
        if !entry?.file_name().to_string_lossy().eq_ignore_ascii_case(DATA_DIR) {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn support_rows(root: &Path, rel: &[String]) -> R<Option<Vec<SupportRow>>> {
    if rel.is_empty() || is_reserved(rel) {
        return Ok(None);
    }
    let (data, _) = load_synced_at(root, rel)?;
    let target = locate(root, &data, rel)?;
    let kinds = kinds_of(root, &data, rel, &target)?;
    if kinds.is_empty() {
        return Ok(None);
    }
    let children = list_subdirs(&target.abs)?;
    let mut rows = Vec::new();
    for kind in kinds {
        let found: Vec<&String> = children.iter().filter(|name| support_kind(name) == Some(kind)).collect();
        if found.is_empty() {
            let name = support_folder_name(kind, &rel[rel.len() - 1], rel.len()).unwrap_or_default();
            rows.push(SupportRow { kind, name, exists: false, empty: true });
        }
        for name in found {
            rows.push(SupportRow { kind, name: name.clone(), exists: true, empty: is_empty(&target.abs.join(name))? });
        }
    }
    Ok(Some(rows))
}

pub fn add_support(root: &Path, rel: &[String], kind: &str) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced_at(root, rel)?;
    let (target, kinds) = editable_holder(root, &data, rel)?;
    let Some(&kind) = kinds.iter().find(|k| **k == kind) else {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: format!("a {kind} folder is not allowed here") });
    };
    let name = support_folder_name(kind, &rel[rel.len() - 1], rel.len()).unwrap_or_default();
    if list_subdirs(&target.abs)?.iter().any(|existing| support_kind(existing) == Some(kind)) {
        return Err(FmError::AlreadyExists { name });
    }
    validate_component(&name)?;
    let parent_yaml = get_node(&data, rel).ok_or_else(|| FmError::NotFound { rel_parts: rel.to_vec() })?;
    ensure_free(&target.abs, Some(parent_yaml), &name, &[])?;
    let mut journal = Journal::new();
    journal.mkdir(&target.abs.join(&name))?;
    // A deepest node's support folder lives on disk only, as the tree stops at the depth limit.
    let tracked = rel.len() < depth::limit(root)?;
    if tracked {
        let mut entry = Mapping::new();
        entry.insert("type".into(), format!("{kind}-L{}", rel.len()).into());
        match get_node_mut(&mut data, rel) {
            Some(node) => {
                node.insert(name.clone().into(), entry.into());
            }
            None => return Err(journal.abort("add support folder", FmError::NotFound { rel_parts: rel.to_vec() })),
        }
    }
    let touched = vec![if tracked { child_rel(rel, &name) } else { rel.to_vec() }];
    persist(root, &data, journal, "add support folder", &scope(&synced, touched.clone()))?;
    Ok(finish(root, &data, false, None, Vec::new(), scope(&synced, touched)))
}

pub fn remove_support(root: &Path, rel: &[String], name: &str, recycle: impl FnOnce(&Path) -> Result<(), String>) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced_at(root, rel)?;
    let (target, kinds) = editable_holder(root, &data, rel)?;
    let child = child_rel(rel, name);
    if !support_kind(name).is_some_and(|kind| kinds.contains(&kind)) {
        return Err(FmError::InvalidPath { rel_parts: child, reason: "not a support folder this node may hold".to_string() });
    }
    if !list_subdirs(&target.abs)?.iter().any(|existing| existing == name) {
        return Err(FmError::NotFound { rel_parts: child });
    }
    let abs = target.abs.join(name);
    if !is_empty(&abs)? {
        return Err(FmError::InvalidPath { rel_parts: child, reason: "it is not empty, so it stays".to_string() });
    }
    let before = subtrees(&data, &[child.clone()]);
    move_to_bin(&abs, recycle)?;
    delete_key(&mut data, rel, name);
    persist(root, &data, Journal::new(), "remove support folder", &scope(&synced, vec![child.clone()]))?;
    Ok(finish(root, &data, false, None, Vec::new(), scope(&synced, vec![child])).replacing(before))
}
