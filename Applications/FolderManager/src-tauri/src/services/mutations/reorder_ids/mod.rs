// Gives a folder's numbered children and worknodes new ids, rotating them through temporary names so no rename collides.
mod plan;
#[cfg(test)]
mod tests;

use std::path::Path;

use serde::Deserialize;
use serde_yaml::Value;

use super::rename::cascade_linked;
use super::{child_rel, editable_target, ensure_free, finish, load_synced_at, persist, scope, subtrees, Mutation};
use crate::domain::error::{FmError, R};
use crate::domain::names::editable_part;
use crate::infra::fs_safe::{join_rel, same_name, validate_component, Journal, StorageLock};
use crate::services::automations::dictionary::rename_node_refs;
use crate::services::tree::tree_store::{self, get_node, get_node_mut};
use plan::{groups, renamed, IdKind};
pub use plan::ReorderPlan;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdTarget {
    pub name: String,
    pub new_id: String,
}

// Read without the lock: the dialog only shows it, and the reorder checks everything again.
pub fn reorder_plan(root: &Path, parent: &[String]) -> R<ReorderPlan> {
    groups(root, &tree_store::load(root)?, parent)
}

pub fn reorder_ids(root: &Path, parent: &[String], targets: &[IdTarget]) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced_at(root, parent)?;
    if !parent.is_empty() {
        editable_target(root, &data, parent, "reorder")?;
    }
    let plan = groups(root, &data, parent)?;
    let mut given: Vec<&String> = targets.iter().map(|t| &t.name).collect();
    given.sort();
    given.dedup();
    let mut known: Vec<&String> = plan.nodes.iter().chain(&plan.worknodes).collect();
    known.sort();
    // A list that is not exactly the listed children means the UI is out of date.
    if given.len() != targets.len() || given != known {
        return Err(FmError::Stale { rel_parts: parent.to_vec() });
    }

    let mut used = plan.taken.clone();
    let mut renames: Vec<(String, String, IdKind)> = Vec::new();
    for target in targets {
        let kind = if plan.nodes.contains(&target.name) { IdKind::Node } else { IdKind::Worknode };
        let (new, key) = renamed(&target.name, kind, parent.is_empty(), &target.new_id)?;
        if used.contains(&key) {
            return Err(FmError::AlreadyExists { name: new });
        }
        used.push(key);
        if new != target.name {
            renames.push((target.name.clone(), new, kind));
        }
    }
    let parent_abs = join_rel(root, parent);
    let leaving: Vec<&str> = renames.iter().map(|(old, _, _)| old.as_str()).collect();
    for (i, (_, new, _)) in renames.iter().enumerate() {
        validate_component(new)?;
        ensure_free(&parent_abs, get_node(&data, parent), new, &leaving)?;
        if renames[..i].iter().any(|(_, other, _)| same_name(other, new)) {
            return Err(FmError::AlreadyExists { name: new.clone() });
        }
    }

    let old_rels: Vec<Vec<String>> = renames.iter().map(|(old, _, _)| child_rel(parent, old)).collect();
    let before = subtrees(&data, &old_rels);
    let temps: Vec<String> = (0..renames.len()).map(|i| format!("~fm-reorder-{}-{i}", std::process::id())).collect();
    let mut journal = Journal::new();
    for ((old, _, _), temp) in renames.iter().zip(&temps) {
        if let Err(e) = journal.rename(&parent_abs.join(old), &parent_abs.join(temp)) {
            return Err(journal.abort("reorder", e));
        }
    }
    for ((_, new, _), temp) in renames.iter().zip(&temps) {
        if let Err(e) = journal.rename(&parent_abs.join(temp), &parent_abs.join(new)) {
            return Err(journal.abort("reorder", e));
        }
    }
    if let Some(siblings) = get_node_mut(&mut data, parent) {
        let entries: Vec<Option<Value>> = renames.iter().map(|(old, _, _)| siblings.remove(old.as_str())).collect();
        for ((_, new, _), entry) in renames.iter().zip(entries) {
            if let Some(entry) = entry {
                siblings.insert(new.clone().into(), entry);
            }
        }
    }
    // A worknode's number is part of its title, so its linked RES./REV. folders follow it as a rename would.
    let mut warnings = Vec::new();
    for (old, new, kind) in &renames {
        if *kind == IdKind::Worknode {
            warnings.extend(cascade_linked(&mut journal, &mut data, &parent_abs.join(new), &child_rel(parent, new), old, new));
        }
    }
    let mut touched: Vec<Vec<String>> = old_rels.into_iter().chain(renames.iter().map(|(_, new, _)| child_rel(parent, new))).collect();
    // Nothing moved, yet the parent level is still patched instead of the whole tree being sent.
    if touched.is_empty() {
        touched = known.iter().map(|s| child_rel(parent, s)).collect();
    }
    persist(root, &data, journal, "reorder", &scope(&synced, touched.clone()))?;
    warnings.extend(follow_dictionaries(root, parent, &renames));
    Ok(finish(root, &data, false, None, warnings, scope(&synced, touched)).replacing(before))
}

// Headings name a worknode by its title with the number, and two of them may swap numbers, so they go through temporary names too.
fn follow_dictionaries(root: &Path, parent: &[String], renames: &[(String, String, IdKind)]) -> Vec<String> {
    let changed: Vec<(&str, &str)> = renames.iter().map(|(old, new, _)| (editable_part(old), editable_part(new))).filter(|(old, new)| old != new).collect();
    let temps: Vec<String> = (0..changed.len()).map(|i| format!("~fm-reorder-{i}")).collect();
    let mut warnings = Vec::new();
    for ((old, _), temp) in changed.iter().zip(&temps) {
        warnings.extend(rename_node_refs(root, parent, old, temp));
    }
    for ((_, new), temp) in changed.iter().zip(&temps) {
        warnings.extend(rename_node_refs(root, parent, temp, new));
    }
    warnings
}
