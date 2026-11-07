// Gives a folder that breaks the id format its place: a review folder, a node or a workboard.
#[cfg(test)]
mod tests;

use std::path::Path;

use serde::Deserialize;
use serde_yaml::{Mapping, Value};

use super::add::mark_workboard;
use super::{child_rel, ensure_free, finish, move_to_review, persist, scope, split, subtrees, Drift, Mutation};
use crate::services::tree::tree_store::load_synced;
use crate::domain::names::{has_node_id, is_track_name, is_valid_root_name, next_available_id, next_main_category_id, next_track_number, sanitize_train_case, title_of, tracker_number, workboard_title};
use crate::infra::scan::list_subdirs;
use crate::domain::tree_types::{child_entries, type_base};
use crate::domain::config::{DATA_FLAG, WKB_IDENTIFIERS};
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::{join_rel, Journal, StorageLock};
use crate::services::tree::tree_store::{self, delete_key, get_node, get_node_mut};

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Resolution {
    Review { destination: Vec<String> },
    Node,
    Workboard { tracker: String },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pending {
    pub rel_parts: Vec<String>,
    pub resolution: Resolution,
}

pub fn resolve_invalid(root: &Path, rel: &[String], resolution: Resolution) -> R<Mutation> {
    match resolution {
        Resolution::Review { destination } => move_to_review(root, rel, &destination),
        Resolution::Node => become_node(root, rel),
        Resolution::Workboard { tracker } => become_workboard(root, rel, &tracker),
    }
}

// The names the rows would get if applied one after another, deepest first as the apply runs, so two rows never share an id.
pub fn preview_names(root: &Path, rows: &[Pending]) -> R<Vec<String>> {
    let mut data = tree_store::load(root)?;
    let mut names: Vec<String> = rows.iter().map(|row| row.rel_parts.last().cloned().unwrap_or_default()).collect();
    let mut order: Vec<usize> = (0..rows.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(rows[i].rel_parts.len()));
    for i in order {
        let rel = &rows[i].rel_parts;
        let (_, parent) = split(rel)?;
        let new = match &rows[i].resolution {
            Resolution::Review { .. } => continue,
            Resolution::Node => node_name(root, &data, rel)?,
            Resolution::Workboard { tracker } => {
                delete_key(&mut data, parent, rel.last().map_or("", String::as_str));
                workboard_name(root, &data, rel, &tracker.to_ascii_uppercase())?
            }
        };
        // The name is held in the tree so the next row sees it as taken.
        if let Some(slot) = get_node_mut(&mut data, parent) {
            slot.insert(new.as_str().into(), Mapping::new().into());
        }
        names[i] = new;
    }
    Ok(names)
}

// Takes the next free id of its level; a root folder becomes a main category.
fn node_name(root: &Path, data: &Mapping, rel: &[String]) -> R<String> {
    let (_, parent) = split(rel)?;
    let title = title_of(rel.last().map_or("", String::as_str));
    let taken = taken_names(root, data, parent)?;
    Ok(if parent.is_empty() {
        format!("{:03}.{title}", next_main_category_id(&taken))
    } else {
        format!("{:02}.{title}", next_available_id(&taken))
    })
}

// Keeps the number the folder already has for that tracker when no sibling uses it, else continues the siblings' count; the title loses its tracker and hash.
fn workboard_name(root: &Path, data: &Mapping, rel: &[String], tracker: &str) -> R<String> {
    let (name, parent) = split(rel)?;
    let title = sanitize_train_case(&workboard_title(name));
    let letters = format!("{tracker}.");
    let siblings: Vec<String> = taken_names(root, data, parent)?.into_iter().filter(|s| s != name).collect();
    let used = |n: u32| siblings.iter().any(|s| tracker_number(s, &letters) == Some(n));
    let id = tracker_number(name, &letters).filter(|n| *n >= 1 && !used(*n)).unwrap_or_else(|| next_track_number(&letters, &siblings));
    Ok(if title.is_empty() { format!("{tracker}.{id:03}") } else { format!("{tracker}.{id:03}_{title}") })
}

fn become_node(root: &Path, rel: &[String]) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, changed) = load_synced(root)?;
    let synced = Drift::unscoped(changed);
    let (_, parent) = split(rel)?;
    refuse_if_node(rel, &node_entry(&data, rel)?)?;
    let new = node_name(root, &data, rel)?;
    let level = rel.len();
    let mut entry = node_entry(&data, rel)?;
    entry.insert("type".into(), format!("NODE-L{level}").into());
    entry.remove("status");
    if level >= 3 {
        entry.insert("flags".into(), DATA_FLAG.into());
    }
    let new_rel = child_rel(parent, &new);
    let before = subtrees(&data, &[rel.to_vec()]);
    reidentify(root, &mut data, (rel, &new_rel), &synced, entry, "make node")?;
    Ok(finish(root, &data, false, Some(new_rel.clone()), Vec::new(), scope(&synced, vec![rel.to_vec(), new_rel])).replacing(before))
}

// Takes the lowest free track of its tracker and the usual info.json of a workboard, keeping any values it already has.
fn become_workboard(root: &Path, rel: &[String], tracker: &str) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, changed) = load_synced(root)?;
    let synced = Drift::unscoped(changed);
    let (_, parent) = split(rel)?;
    let tracker = tracker.to_ascii_uppercase();
    let tracker = tracker.as_str();
    if parent.is_empty() {
        return Err(refuse(rel, "workboards cannot be created at the storage root"));
    }
    if !WKB_IDENTIFIERS.contains(&format!("{tracker}.").as_str()) {
        return Err(refuse(rel, format!("{tracker} is not a workboard tracker")));
    }
    let mut entry = node_entry(&data, rel)?;
    refuse_if_workboard(rel, &entry)?;
    let new = workboard_name(root, &data, rel, tracker)?;
    let status = entry.get("status").and_then(Value::as_str).unwrap_or("in-progress").to_string();
    entry.insert("type".into(), format!("WKB-L{}", rel.len()).into());
    entry.remove("flags");
    let new_rel = child_rel(parent, &new);
    let before = subtrees(&data, &[rel.to_vec()]);
    reidentify(root, &mut data, (rel, &new_rel), &synced, entry, "make workboard")?;
    let mut warnings = Vec::new();
    if let Err(e) = mark_workboard(&join_rel(root, &new_rel), parent.last().map_or("", String::as_str), &status) {
        warnings.push(format!("Could not set up the info file: {e}"));
    }
    Ok(finish(root, &data, false, Some(new_rel.clone()), warnings, scope(&synced, vec![rel.to_vec(), new_rel])).replacing(before))
}

fn refuse(rel: &[String], reason: impl Into<String>) -> FmError {
    FmError::InvalidPath { rel_parts: rel.to_vec(), reason: reason.into() }
}

fn kind_of(node: &Mapping) -> &str {
    node.get("type").and_then(Value::as_str).unwrap_or_default()
}

// A node keeps the id it has; a workboard-typed folder with a bad name is still free to become a node.
fn refuse_if_node(rel: &[String], node: &Mapping) -> R<()> {
    let name = rel.last().map_or("", String::as_str);
    let has_id = if rel.len() == 1 { is_valid_root_name(name) } else { has_node_id(name) };
    if type_base(kind_of(node)) == "NODE" && has_id {
        return Err(refuse(rel, "the folder already has an id"));
    }
    Ok(())
}

fn refuse_if_workboard(rel: &[String], node: &Mapping) -> R<()> {
    if type_base(kind_of(node)) == "WKB" && is_track_name(rel.last().map_or("", String::as_str)) {
        return Err(refuse(rel, "the folder already has a track"));
    }
    Ok(())
}

// The names a new id must avoid: folders on disk and tree entries under the parent.
fn taken_names(root: &Path, data: &Mapping, parent: &[String]) -> R<Vec<String>> {
    let mut names = list_subdirs(&join_rel(root, parent))?;
    let node = get_node(data, parent).ok_or_else(|| FmError::NotFound { rel_parts: parent.to_vec() })?;
    names.extend(child_entries(node).map(|(name, _)| name.to_string()));
    Ok(names)
}

fn node_entry(data: &Mapping, rel: &[String]) -> R<Mapping> {
    get_node(data, rel).cloned().ok_or_else(|| FmError::NotFound { rel_parts: rel.to_vec() })
}

// Renames the folder and moves its tree entry with it; the disk rename is rolled back when the tree cannot be saved.
fn reidentify(root: &Path, data: &mut Mapping, (rel, new_rel): (&[String], &[String]), synced: &Drift, entry: Mapping, op: &str) -> R<()> {
    let (old, parent) = split(rel)?;
    let (new, _) = split(new_rel)?;
    ensure_free(&join_rel(root, parent), get_node(data, parent), new, &[old.as_str()])?;
    let mut journal = Journal::new();
    journal.rename(&join_rel(root, rel), &join_rel(root, new_rel))?;
    delete_key(data, parent, old);
    match get_node_mut(data, parent) {
        Some(slot) => {
            slot.insert(new.as_str().into(), entry.into());
        }
        None => return Err(journal.abort(op, FmError::NotFound { rel_parts: parent.to_vec() })),
    }
    persist(root, data, journal, op, &scope(synced, vec![rel.to_vec(), new_rel.to_vec()]))
}
