// Gives a folder that breaks the id format its place: a review folder, a node or a workboard.
use std::path::Path;

use serde::Deserialize;
use serde_yaml::{Mapping, Value};

use super::add::{mark_workboard, track_or_lowest};
use super::{child_rel, ensure_free, finish, load_synced, move_to_review, persist, split, subtrees, Mutation};
use crate::ant_core::names::{has_node_id, is_track_name, is_valid_root_name, list_subdirs, next_available_id, next_main_category_id, sanitize_train_case, title_of, tracker_number, workboard_title};
use crate::ant_core::tree_types::{child_entries, type_base};
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
            Resolution::Workboard { tracker } => {
                delete_key(&mut data, parent, rel.last().map_or("", String::as_str));
                workboard_name(&data, rel, &tracker.to_ascii_uppercase())
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

// Keeps the number the folder already has for that tracker when it is free, else takes the lowest free one; the title loses its tracker and hash.
fn workboard_name(data: &Mapping, rel: &[String], tracker: &str) -> String {
    let name = rel.last().map_or("", String::as_str);
    let title = sanitize_train_case(&workboard_title(name));
    let id = track_or_lowest(data, tracker, tracker_number(name, &format!("{tracker}.")));
    if title.is_empty() { format!("{tracker}.{id:03}") } else { format!("{tracker}.{id:03}_{title}") }
}

fn become_node(root: &Path, rel: &[String]) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced(root)?;
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
    reidentify(root, &mut data, rel, &new_rel, entry, "make node")?;
    Ok(finish(root, &data, synced, Some(new_rel.clone()), Vec::new(), vec![rel.to_vec(), new_rel]).replacing(before))
}

// Takes the lowest free track of its tracker and the usual info.json of a workboard, keeping any values it already has.
fn become_workboard(root: &Path, rel: &[String], tracker: &str) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced(root)?;
    let (old, parent) = split(rel)?;
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
    // The folder's own entry is left out, so the number it holds now counts as free.
    let mut others = data.clone();
    delete_key(&mut others, parent, old);
    let new = workboard_name(&others, rel, tracker);
    let status = entry.get("status").and_then(Value::as_str).unwrap_or("in-progress").to_string();
    entry.insert("type".into(), format!("WKB-L{}", rel.len()).into());
    entry.remove("flags");
    let new_rel = child_rel(parent, &new);
    let before = subtrees(&data, &[rel.to_vec()]);
    reidentify(root, &mut data, rel, &new_rel, entry, "make workboard")?;
    let mut warnings = Vec::new();
    if let Err(e) = mark_workboard(&join_rel(root, &new_rel), parent.last().map_or("", String::as_str), &status) {
        warnings.push(format!("Could not set up the info file: {e}"));
    }
    Ok(finish(root, &data, synced, Some(new_rel.clone()), warnings, vec![rel.to_vec(), new_rel]).replacing(before))
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
    fn a_workboard_or_track_turned_into_a_node_keeps_one_id() {
        let tree = "100.ACADEMIC:\n  type: NODE-L1\n  01.New-Node: {type: WKB-L2}\n  PJ.1007_Prueba-7: {type: NODE-L2}\n";
        let dir = storage("resolve-node-id", &["100.ACADEMIC/01.New-Node", "100.ACADEMIC/PJ.1007_Prueba-7"], tree);
        resolve_invalid(dir.path(), &rel(&["100.ACADEMIC", "01.New-Node"]), Resolution::Node).unwrap();
        assert!(dir.path().join("100.ACADEMIC/02.New-Node").is_dir());
        resolve_invalid(dir.path(), &rel(&["100.ACADEMIC", "PJ.1007_Prueba-7"]), Resolution::Node).unwrap();
        assert!(dir.path().join("100.ACADEMIC/01.Prueba-7").is_dir());
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "02.New-Node"]), "NODE-L2");
        assert!(crate::tree::root_check::check_root(dir.path()).unwrap().invalid.is_empty());
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
    fn a_workboard_with_a_bad_name_takes_the_lowest_free_track_and_keeps_its_title() {
        let tree = "100.ACADEMIC:\n  type: NODE-L1\n  01.New-Node: {type: WKB-L2}\n  PJ.001_Fine: {type: WKB-L2}\n  PJ.1007_Prueba-7: {type: NODE-L2}\n";
        let dir = storage("resolve-track", &["100.ACADEMIC/01.New-Node", "100.ACADEMIC/PJ.001_Fine", "100.ACADEMIC/PJ.1007_Prueba-7"], tree);
        let pj = Resolution::Workboard { tracker: "PJ".to_string() };
        resolve_invalid(dir.path(), &rel(&["100.ACADEMIC", "01.New-Node"]), pj).unwrap();
        let folder = dir.path().join("100.ACADEMIC/PJ.002_New-Node");
        assert!(folder.is_dir());
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "PJ.002_New-Node"]), "WKB-L2");
        assert_eq!(field_of(&read_info(&folder).unwrap(), &["status"]), Some("in-progress"));
        let pj = Resolution::Workboard { tracker: "PJ".to_string() };
        resolve_invalid(dir.path(), &rel(&["100.ACADEMIC", "PJ.1007_Prueba-7"]), pj).unwrap();
        assert!(dir.path().join("100.ACADEMIC/PJ.1007_Prueba-7").is_dir());
    }

    #[test]
    fn a_tracker_in_the_wrong_case_is_normalized_and_its_hash_is_dropped() {
        let old = "Pj.154_Google-Labs_Agent-9f0d172a";
        let tree = format!("100.ACADEMIC:\n  type: NODE-L1\n  {old}: {{type: NODE-L2}}\n");
        let dir = storage("resolve-case", &[&format!("100.ACADEMIC/{old}")], &tree);
        let pj = || Resolution::Workboard { tracker: "Pj".to_string() };
        let row = Pending { rel_parts: rel(&["100.ACADEMIC", old]), resolution: pj() };
        assert_eq!(preview_names(dir.path(), &[row]).unwrap(), ["PJ.154_Google-Labs-Agent"]);
        resolve_invalid(dir.path(), &rel(&["100.ACADEMIC", old]), pj()).unwrap();
        assert!(dir.path().join("100.ACADEMIC/PJ.154_Google-Labs-Agent").is_dir());
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "PJ.154_Google-Labs-Agent"]), "WKB-L2");
    }

    #[test]
    fn a_number_another_workboard_uses_gives_way_to_the_lowest_free_one() {
        let tree = "100.ACADEMIC:\n  type: NODE-L1\n  PJ.154_Other: {type: WKB-L2}\n  Pj.154_Mine: {type: NODE-L2}\n";
        let dir = storage("resolve-taken", &["100.ACADEMIC/PJ.154_Other", "100.ACADEMIC/Pj.154_Mine"], tree);
        resolve_invalid(dir.path(), &rel(&["100.ACADEMIC", "Pj.154_Mine"]), Resolution::Workboard { tracker: "PJ".to_string() }).unwrap();
        assert!(dir.path().join("100.ACADEMIC/PJ.001_Mine").is_dir());
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