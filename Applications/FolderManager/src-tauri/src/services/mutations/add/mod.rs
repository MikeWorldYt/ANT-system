// A new folder is created with its tree entry, 98 review folder and .ant marker as one journaled step.
#[cfg(test)]
mod tests;

use std::fs;
use std::path::Path;

use serde_yaml::{Mapping, Value};

use super::{child_rel, ensure_free, finish, guard_frozen, load_synced_at, locate, persist, register_ids, scope, Mutation};
use crate::domain::names::{editable_part, next_available_id, next_main_category_id, next_track_number};
use crate::infra::scan::list_subdirs;
use crate::domain::tree_types::{child_entries, META_KEYS};
use crate::services::backup::stamp::local_date;
use crate::domain::config::{NewKind, DATA_DIR, DATA_FLAG, OWNER_KEYS, REVIEW_PREFIX};
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::{io_err, resolve, Journal, StorageLock};
use crate::infra::naming::free_name;
use crate::services::tree::tree_node::is_reserved;
use crate::services::tree::tree_store;
use crate::infra::storage::db::hide;
use crate::infra::storage::info::{field_of, read_info, write_info};
use crate::services::tree::depth;
use crate::services::tree::tree_store::{get_node, get_node_mut};

// A selected Level 1 node creates a sibling category at the Storage root.
pub fn add_next(root: &Path, rel: &[String], kind: NewKind) -> R<Mutation> {
    if rel.is_empty() {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "the storage root is not a node".to_string() });
    }
    if rel.len() == 1 && kind == NewKind::Workboard {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "workboards cannot be created at the storage root".to_string() });
    }
    let parent = if rel.len() == 1 { &[][..] } else { &rel[..rel.len() - 1] };
    add_node(root, parent, kind)
}

pub fn add_node(root: &Path, parent: &[String], kind: NewKind) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    if !parent.is_empty() && parent.len() >= depth::limit(root)? {
        return add_past_limit(root, parent, kind);
    }
    let (mut data, synced) = load_synced_at(root, parent)?;
    if parent.is_empty() && kind == NewKind::Workboard {
        return Err(FmError::InvalidPath { rel_parts: parent.to_vec(), reason: "workboards cannot be created at the storage root".to_string() });
    }
    let target = if parent.is_empty() {
        super::Target { abs: root.to_path_buf(), ghost: false, node_type: String::new() }
    } else {
        locate(root, &data, parent)?
    };
    guard_frozen(parent, &target.node_type)?;
    if target.ghost {
        return Err(FmError::GhostNotAllowed { op: "add node".to_string() });
    }
    let limit = depth::limit(root)?;
    let child_depth = parent.len();
    if child_depth >= limit {
        return Err(FmError::DepthLimit);
    }
    let parent_yaml =
        get_node(&data, parent).ok_or_else(|| FmError::NotFound { rel_parts: parent.to_vec() })?;
    // Ghost ids count as taken so a new node never inherits a missing folder's record.
    let mut taken = list_subdirs(&target.abs)?;
    taken.extend(child_entries(parent_yaml).map(|(k, _)| k.to_string()));
    let workboard = kind == NewKind::Workboard;
    let new_name = if parent.is_empty() {
        format!("{:03}.NEW-CATEGORY", next_main_category_id(&taken))
    } else if workboard {
        format!("PJ.{:03}_New-Workboard", next_track_number("PJ.", &taken))
    } else {
        format!("{:02}.New-Node", next_available_id(&taken))
    };
    ensure_free(&target.abs, Some(parent_yaml), &new_name, &[])?;

    let new_abs = target.abs.join(&new_name);
    let mut journal = Journal::new();
    journal.mkdir(&new_abs)?;
    let level = child_depth + 1;
    let mut entry = Mapping::new();
    entry.insert("type".into(), format!("{}-L{level}", if workboard { "WKB" } else { "NODE" }).into());
    if !workboard && level >= 3 {
        entry.insert("flags".into(), DATA_FLAG.into());
    }
    let mut warnings = Vec::new();
    let rev_name = format!("{REVIEW_PREFIX}{}", editable_part(&new_name));
    // A workboard keeps its review shortcuts out of its own folder.
    match if workboard { Ok(()) } else { journal.mkdir(&new_abs.join(&rev_name)) } {
        // The deepest level keeps its review folder on disk only, as the tree stops at the depth limit.
        Ok(()) if level < limit && !workboard => {
            let mut rev = Mapping::new();
            rev.insert("type".into(), format!("REV-L{level}").into());
            entry.insert(rev_name.into(), rev.into());
        }
        Ok(()) => {}
        Err(e) => warnings.push(format!("Could not create {rev_name}: {e}")),
    }
    match get_node_mut(&mut data, parent) {
        Some(node) => {
            node.insert(new_name.clone().into(), entry.into());
        }
        None => {
            let cause = FmError::NotFound { rel_parts: parent.to_vec() };
            return Err(journal.abort("add node", cause));
        }
    }
    persist(root, &data, journal, "add node", &scope(&synced, vec![child_rel(parent, &new_name)]))?;
    // The folder exists now, so a failure here is only reported.
    if let Err(e) = mark_folder(&new_abs, workboard, parent.last().map_or("", String::as_str)) {
        warnings.push(format!("Could not set up {DATA_DIR}: {e}"));
    } else if workboard {
        let defaults = get_node(&data, parent).map_or(Ok(()), |node| super::automations::apply_defaults(&target.abs, node, &new_abs));
        if let Err(e) = defaults {
            warnings.push(format!("Could not apply the Outcomes defaults: {e}"));
        }
    }
    let moved = register_ids(root, &mut data, &mut warnings);
    let new_rel = child_rel(parent, &new_name);
    Ok(finish(root, &data, false, Some(new_rel.clone()), warnings, scope(&synced.or_unscoped(moved), vec![new_rel])))
}

// Past the depth limit the database tracks nothing, so a node is a plain folder.
fn add_past_limit(root: &Path, parent: &[String], kind: NewKind) -> R<Mutation> {
    if is_reserved(&parent[..1]) {
        return Err(FmError::Reserved { rel_parts: parent.to_vec() });
    }
    if let Some(meta) = parent.iter().find(|p| META_KEYS.contains(&p.as_str())) {
        return Err(FmError::InvalidName { name: meta.clone(), reason: "reserved name".to_string() });
    }
    let data = tree_store::load(root)?;
    guard_frozen(parent, &tree_store::get_type(&data, parent))?;
    let dir = resolve(root, parent)?;
    if !dir.is_dir() {
        return Err(FmError::NotFound { rel_parts: parent.to_vec() });
    }
    let workboard = kind == NewKind::Workboard;
    let name = if workboard {
        format!("PJ.{:03}_New-Workboard", next_track_number("PJ.", &list_subdirs(&dir)?))
    } else {
        free_name(&dir, "New folder", &[])?
    };
    let abs = dir.join(&name);
    fs::create_dir(&abs).map_err(|e| io_err("create folder", &abs, &e))?;
    let mut warnings = Vec::new();
    if workboard {
        if let Err(e) = mark_folder(&abs, true, parent.last().map_or("", String::as_str)) {
            warnings.push(format!("Could not set up {DATA_DIR}: {e}"));
        }
    }
    let new_rel = child_rel(parent, &name);
    Ok(finish(root, &data, false, Some(new_rel.clone()), warnings, vec![new_rel]))
}

// Every created folder gets a .ant folder, which is what lets its id follow it around; workboards also start their info.json.
pub(super) fn mark_folder(folder: &Path, workboard: bool, parent: &str) -> R<()> {
    if workboard {
        return mark_workboard(folder, parent, "in-progress");
    }
    let dir = folder.join(DATA_DIR);
    fs::create_dir(&dir).map_err(|e| io_err("create info folder", &dir, &e))?;
    hide(&dir);
    Ok(())
}

// Only the keys the info.json lacks get their default, so a workboard's own status and owner stay.
pub(super) fn mark_workboard(folder: &Path, parent: &str, status: &str) -> R<()> {
    let current = read_info(folder);
    let lacks = |keys: &[&str]| current.as_ref().and_then(|info| field_of(info, keys)).is_none();
    let mut updates: Vec<(&[&str], &str, Option<Value>)> = Vec::new();
    if lacks(&["status"]) {
        updates.push((&["status"], "status", Some(status.into())));
    }
    if lacks(&OWNER_KEYS) {
        updates.push((&OWNER_KEYS, "owner", Some("unknown".into())));
    }
    if lacks(&["created"]) {
        updates.push((&["created"], "created", Some(local_date().into())));
    }
    if lacks(&["breadcrumb"]) {
        updates.push((&["breadcrumb"], "breadcrumb", Some(parent.into())));
    }
    if updates.is_empty() {
        return Ok(());
    }
    write_info(folder, &updates)
}
