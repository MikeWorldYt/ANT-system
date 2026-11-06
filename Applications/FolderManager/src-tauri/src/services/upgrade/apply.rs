// Settles one numeric support folder: a support one gets its word prefix, any other the next free regular id.
use std::path::Path;

use serde_yaml::Value;

use super::scan::is_support;
use crate::domain::config::SUPPORT_NODES;
use crate::domain::error::{FmError, R};
use crate::domain::names::{editable_part, is_legacy_support, next_available_id};
use crate::domain::tree_types::type_base;
use crate::infra::fs_safe::{join_rel, validate_component, Journal, StorageLock};
use crate::infra::scan::list_subdirs;
use crate::services::mutations::{child_rel, ensure_free, finish, load_synced_at, locate, parent_of, persist, scope, split, subtrees, Mutation};
use crate::services::tree::tree_store::{get_node, get_node_mut, rename_key};

// A folder that is kept becomes a regular node because no 90/97/98 id is a node id, so it takes the lowest free one.
pub fn apply_support_upgrade(root: &Path, rel: &[String]) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    rel.iter().try_for_each(|part| validate_component(part))?;
    let (old_name, parent) = split(rel)?;
    if rel.len() < 2 || !is_legacy_support(old_name) {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "not a numeric support folder".to_string() });
    }
    let keep = !is_support(parent.last().map_or("", String::as_str), old_name);
    let (mut data, synced) = load_synced_at(root, parent_of(rel))?;
    let target = locate(root, &data, rel)?;
    if target.ghost {
        return Err(FmError::GhostNotAllowed { op: "upgrade".to_string() });
    }
    let parent_abs = join_rel(root, parent);
    let mut taken = list_subdirs(&parent_abs)?;
    let yaml_names: Vec<String> = get_node(&data, parent).map(|n| n.keys().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default();
    taken.extend(yaml_names.iter().cloned());
    let editable = editable_part(old_name);
    let (word, kind) = SUPPORT_NODES.iter().find(|(_, legacy, _)| old_name.starts_with(legacy)).map(|(word, _, kind)| (*word, *kind)).unwrap_or(("", ""));
    let wanted = if keep { format!("{:02}.{editable}", next_available_id(&taken)) } else { format!("{word}{editable}") };
    let new_name = (1..).map(|n| if n == 1 { wanted.clone() } else { format!("{wanted}-{n}") }).find(|name| ensure_free(&parent_abs, get_node(&data, parent), name, &[]).is_ok()).unwrap_or(wanted);
    let new_rel = child_rel(parent, &new_name);
    let before = subtrees(&data, &[rel.to_vec()]);
    let mut journal = Journal::new();
    journal.rename(&target.abs, &parent_abs.join(&new_name))?;
    rename_key(&mut data, parent, old_name, &new_name);
    if let Some(node) = get_node_mut(&mut data, &new_rel) {
        let current = node.get("type").and_then(Value::as_str).unwrap_or("");
        let new_type = if keep {
            format!("NODE-L{}", rel.len())
        } else if type_base(current) == kind {
            current.to_string()
        } else if kind == "REV" {
            format!("REV-L{}", rel.len() - 1)
        } else {
            kind.to_string()
        };
        node.insert("type".into(), new_type.into());
    }
    let touched = scope(&synced, vec![rel.to_vec(), new_rel.clone()]);
    persist(root, &data, journal, "support upgrade", &touched)?;
    Ok(finish(root, &data, false, Some(new_rel), Vec::new(), touched).replacing(before))
}
