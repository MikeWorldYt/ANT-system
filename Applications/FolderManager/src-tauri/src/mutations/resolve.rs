// Gives a folder that breaks the id format its place: a review folder, a node or a workboard.
use std::path::Path;

use serde::Deserialize;
use serde_yaml::Mapping;

use super::add::{mark_folder, next_tracker};
use super::{child_rel, ensure_free, finish, load_synced, move_to_review, persist, split, subtrees, Mutation};
use crate::ant_core::names::{has_valid_id, list_subdirs, next_available_id, next_main_category_id, sanitize_train_case};
use crate::ant_core::tree_types::child_entries;
use crate::config::{DATA_FLAG, WKB_IDENTIFIERS};
use crate::error::{FmError, R};
use crate::fs_safe::{join_rel, Journal, StorageLock};
use crate::tree::tree_store::{self, delete_key, get_node, get_node_mut};

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
            Resolution::Workboard { tracker } => workboard_name(&data, rel, tracker),
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
    let (old, parent) = split(rel)?;
    let taken = taken_names(root, data, parent)?;
    Ok(if parent.is_empty() {
        format!("{:03}.{old}", next_main_category_id(&taken))
    } else {
        format!("{:02}.{old}", next_available_id(&taken))
    })
}

// Takes the next free id of its tracker, with the title in the tracker format.
fn workboard_name(data: &Mapping, rel: &[String], tracker: &str) -> String {
    let old = rel.last().map_or("", String::as_str);
    let id = next_tracker(data, tracker);
    let title = sanitize_train_case(&old.replace('_', "-"));
    if title.is_empty() { format!("{tracker}.{id:03}") } else { format!("{tracker}.{id:03}_{title}") }
}

fn become_node(root: &Path, rel: &[String]) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced(root)?;
    let (_, parent) = split(rel)?;
    refuse_if_identified(rel)?;
    let new = node_name(root, &data, rel)?;
    let level = rel.len();
    let mut entry = node_entry(&data, rel)?;
    entry.insert("type".into(), format!("NODE-L{level}").into());
    if level >= 3 {
        entry.insert("flags".into(), DATA_FLAG.into());
    }
    let new_rel = child_rel(parent, &new);
    let before = subtrees(&data, &[rel.to_vec()]);
    reidentify(root, &mut data, rel, &new_rel, entry, "make node")?;
    Ok(finish(root, &data, synced, Some(new_rel.clone()), Vec::new(), vec![rel.to_vec(), new_rel]).replacing(before))
}

// Takes the next free id of its tracker and the usual info.json of a workboard, in the tracker format.
fn become_workboard(root: &Path, rel: &[String], tracker: &str) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced(root)?;
    let (_, parent) = split(rel)?;
    if parent.is_empty() {
        return Err(refuse(rel, "workboards cannot be created at the storage root"));
    }
    if !WKB_IDENTIFIERS.contains(&format!("{tracker}.").as_str()) {
        return Err(refuse(rel, format!("{tracker} is not a workboard tracker")));
    }
    refuse_if_identified(rel)?;
    let new = workboard_name(&data, rel, tracker);
    let mut entry = node_entry(&data, rel)?;
    entry.insert("type".into(), format!("WKB-L{}", rel.len()).into());
    entry.remove("flags");
    let new_rel = child_rel(parent, &new);
    let before = subtrees(&data, &[rel.to_vec()]);
    reidentify(root, &mut data, rel, &new_rel, entry, "make workboard")?;
    let mut warnings = Vec::new();
    if let Err(e) = mark_folder(&join_rel(root, &new_rel), true, parent.last().map_or("", String::as_str)) {
        warnings.push(format!("Could not set up the info file: {e}"));
    }
    Ok(finish(root, &data, synced, Some(new_rel.clone()), warnings, vec![rel.to_vec(), new_rel]).replacing(before))
}

fn refuse(rel: &[String], reason: impl Into<String>) -> FmError {
    FmError::InvalidPath { rel_parts: rel.to_vec(), reason: reason.into() }
}

fn refuse_if_identified(rel: &[String]) -> R<()> {
    if has_valid_id(rel) {
        return Err(refuse(rel, "the folder already has an id"));
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
fn reidentify(root: &Path, data: &mut Mapping, rel: &[String], new_rel: &[String], entry: Mapping, op: &str) -> R<()> {
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
    persist(root, data, journal, op)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::info::{field_of, read_info};
    use crate::test_support::{rel, storage, yaml, yaml_type};
    use crate::tree::tree_store::get_node;
    use serde_yaml::Value;

    const TREE: &str = "100.ACADEMIC:\n  type: NODE-L1\n  01.Math: {type: NODE-L2, Deep: {type: NODE-L3}}\n";

    #[test]
    fn a_node_below_the_root_takes_the_next_free_id_of_its_level() {
        let dir = storage("resolve-node", &["100.ACADEMIC/Physics", "100.ACADEMIC/01.Math/Deep"], TREE);
        resolve_invalid(dir.path(), &rel(&["100.ACADEMIC", "Physics"]), Resolution::Node).unwrap();
        assert!(dir.path().join("100.ACADEMIC/02.Physics").is_dir());
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "02.Physics"]), "NODE-L2");
        resolve_invalid(dir.path(), &rel(&["100.ACADEMIC", "01.Math", "Deep"]), Resolution::Node).unwrap();
        assert!(dir.path().join("100.ACADEMIC/01.Math/01.Deep").is_dir());
        let data = yaml(dir.path());
        let deep = get_node(&data, &rel(&["100.ACADEMIC", "01.Math", "01.Deep"])).unwrap();
        assert_eq!(deep.get("flags").and_then(Value::as_str), Some(DATA_FLAG));
    }

    #[test]
    fn a_root_folder_becomes_the_next_main_category() {
        let dir = storage("resolve-root", &["Marcos"], "");
        resolve_invalid(dir.path(), &rel(&["Marcos"]), Resolution::Node).unwrap();
        assert!(dir.path().join("100.Marcos").is_dir());
    }

    #[test]
    fn a_folder_becomes_a_workboard_with_its_info_file() {
        let dir = storage("resolve-wkb", &["100.ACADEMIC/Thesis"], "100.ACADEMIC:\n  type: NODE-L1\n");
        resolve_invalid(dir.path(), &rel(&["100.ACADEMIC", "Thesis"]), Resolution::Workboard { tracker: "PJ".to_string() }).unwrap();
        let folder = dir.path().join("100.ACADEMIC/PJ.001_Thesis");
        assert!(folder.is_dir());
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "PJ.001_Thesis"]), "WKB-L2");
        let info = read_info(&folder).unwrap();
        assert_eq!(field_of(&info, &["status"]), Some("in-progress"));
    }

    #[test]
    fn the_preview_gives_each_row_its_own_id_in_the_order_the_apply_takes() {
        let dir = storage("resolve-preview", &["100.ACADEMIC/Prueba 1", "100.ACADEMIC/Prueba 2", "100.ACADEMIC/Thesis"], "100.ACADEMIC:\n  type: NODE-L1\n");
        let rows = [
            Pending { rel_parts: rel(&["100.ACADEMIC", "Prueba 1"]), resolution: Resolution::Node },
            Pending { rel_parts: rel(&["100.ACADEMIC", "Prueba 2"]), resolution: Resolution::Node },
            Pending { rel_parts: rel(&["100.ACADEMIC", "Thesis"]), resolution: Resolution::Workboard { tracker: "PJ".to_string() } },
        ];
        assert_eq!(preview_names(dir.path(), &rows).unwrap(), ["01.Prueba 1", "02.Prueba 2", "PJ.001_Thesis"]);
        assert!(dir.path().join("100.ACADEMIC/Prueba 1").is_dir());
    }

    #[test]
    fn identified_folders_and_root_workboards_are_refused() {
        let dir = storage("resolve-refuse", &["100.ACADEMIC/01.Math", "100.ACADEMIC/Thesis"], TREE);
        assert!(resolve_invalid(dir.path(), &rel(&["100.ACADEMIC", "01.Math"]), Resolution::Node).is_err());
        assert!(resolve_invalid(dir.path(), &rel(&["100.ACADEMIC"]), Resolution::Node).is_err());
        assert!(resolve_invalid(dir.path(), &rel(&["100.ACADEMIC", "Thesis"]), Resolution::Workboard { tracker: "XX".to_string() }).is_err());
        assert!(resolve_invalid(dir.path(), &rel(&["Thesis"]), Resolution::Workboard { tracker: "PJ".to_string() }).is_err());
        assert!(dir.path().join("100.ACADEMIC/Thesis").is_dir());
    }
}
