// Rename, retype and add nodes, keeping disk folders, linked 90./98. folders and the tree in step.
mod add;
mod delete;
mod favorite;
mod recycle;
mod rename;
mod reorder;
mod retype;
#[cfg(not(windows))]
mod unix_trash;

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_yaml::{Mapping, Value};

use crate::ant_core::tree_types::META_KEYS;
use crate::config::is_hidden_dir;
use crate::error::{FmError, R};
use crate::fs_safe::{io_err, resolve, same_name, Journal};
use crate::index::footprint;
use crate::tree::tree_node::{build_tree, fill_children, is_reserved, TreeSnapshot};
use crate::tree::tree_store::{self, get_node, get_type, is_locked_type};

pub use add::add_next;
pub use delete::{delete_node, inspect_delete, DeletePreview};
pub use favorite::set_favorite;
pub use recycle::{recover_trashed, recycle_staged, PendingRecycle};
pub use rename::rename_node;
pub use reorder::{move_node, MoveDirection};
pub use retype::{set_details, set_node_type};
#[cfg(test)]
pub(crate) use {add::add_child, recycle::recycle_staged_with};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mutation {
    pub tree: TreeSnapshot,
    pub select: Option<Vec<String>>,
    pub warnings: Vec<String>,
}

pub(crate) struct Target {
    pub abs: PathBuf,
    pub ghost: bool,
    pub node_type: String,
}

// Folders that just got a .ant folder receive their id now instead of at the next refresh.
fn register_ids(root: &Path, data: &mut Mapping, warnings: &mut Vec<String>) {
    match footprint::plan(root, data) {
        Ok(applied) => {
            // The db keeps the old paths if this save fails, so the next run migrates again.
            let saved = if applied.changed { tree_store::save(root, data) } else { Ok(()) };
            if let Err(e) = saved.and_then(|()| applied.commit(root)) {
                warnings.push(format!("Could not save moved folders: {e}"));
            }
        }
        Err(e) => warnings.push(format!("Could not assign folder ids: {e}")),
    }
}

// Folders added on disk since the last refresh get their yaml entry before any check runs.
pub(crate) fn load_synced(root: &Path) -> R<(Mapping, bool)> {
    let mut data = tree_store::load(root)?;
    let changed = tree_store::sync(root, &mut data)?;
    Ok((data, changed))
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
    let mut on_disk = true;
    let mut dir = root.to_path_buf();
    for part in rel {
        if !has_exact_dir(&dir, part)? {
            on_disk = false;
            break;
        }
        dir.push(part);
    }
    if !on_disk && get_node(data, rel).is_none() {
        return Err(FmError::NotFound { rel_parts: rel.to_vec() });
    }
    Ok(Target { abs, ghost: !on_disk, node_type: get_type(data, rel) })
}

// Exact match, because Windows would also open "01.math" for a yaml key "01.Math".
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

pub(crate) fn persist(root: &Path, data: &Mapping, journal: Journal, op: &str) -> R<()> {
    match tree_store::save(root, data) {
        Ok(()) => {
            journal.commit();
            Ok(())
        }
        Err(e) => Err(journal.abort(op, e)),
    }
}

// Runs after the yaml is committed, so it never fails: the change is already on disk.
pub(crate) fn finish(
    root: &Path,
    data: &Mapping,
    yaml_changed: bool,
    select: Option<Vec<String>>,
    warnings: Vec<String>,
) -> Mutation {
    let tree = build_tree(root, data, yaml_changed).and_then(|mut tree| {
        fill_children(root, data, &mut tree.nodes)?;
        Ok(tree)
    });
    after_commit(root, tree, select, warnings)
}

// An empty tree with yamlChanged set tells the UI to refresh instead of showing a stale tree.
fn after_commit(
    root: &Path,
    tree: R<TreeSnapshot>,
    select: Option<Vec<String>>,
    mut warnings: Vec<String>,
) -> Mutation {
    let tree = tree.unwrap_or_else(|e| {
        warnings.push(format!("The change was saved but the tree could not be reloaded; refresh. {e}"));
        TreeSnapshot { root: root.to_string_lossy().into_owned(), nodes: Vec::new(), yaml_changed: true }
    });
    Mutation { tree, select, warnings }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{EditableType, NewKind};
    use crate::fs_safe::StorageLock;
    use crate::test_support::*;

    #[test]
    fn a_mutation_returns_every_level_the_snapshot_shows() {
        let dir = academic("finish-depth");
        let m = rename_node(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), "Chem").unwrap();
        let kids = |nodes: &[crate::tree::tree_node::TreeNode], name: &str| {
            nodes.iter().find(|n| n.name == name).and_then(|n| n.children.clone()).unwrap_or_default()
        };
        let academic = kids(&m.tree.nodes, "100.ACADEMIC");
        let math = kids(&academic, "01.Math");
        assert!(math.iter().any(|n| n.name == "01.Algebra"), "{math:?}");
        assert!(academic.iter().any(|n| n.name == "02.Chem"));
    }

    #[test]
    fn meta_key_folders_are_refused_and_the_parent_type_survives() {
        let dir = academic("meta-key");
        dir.mkdir("100.ACADEMIC/type");
        dir.mkdir("100.ACADEMIC/01.Math/status");
        let refused = |r: R<Mutation>| matches!(r, Err(FmError::InvalidName { .. }));
        assert!(refused(rename_node(dir.path(), &rel(&["100.ACADEMIC", "type"]), "X")));
        assert!(refused(set_node_type(dir.path(), &rel(&["100.ACADEMIC", "type"]), EditableType::Wkb, None)));
        assert!(refused(add_child(dir.path(), &rel(&["100.ACADEMIC", "01.Math", "status"]), NewKind::Node)));
        assert!(dir.path().join("100.ACADEMIC/type").is_dir());
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC"]), "NODE-L1");
        let math = get_node(&yaml(dir.path()), &rel(&["100.ACADEMIC", "01.Math"])).unwrap().clone();
        assert_eq!(math.get("status").and_then(Value::as_str), Some("done"));
    }

    #[test]
    fn locate_needs_every_component_with_its_exact_case() {
        let dir = academic("locate-case");
        let data: Mapping = serde_yaml::from_str("100.academic:\n  02.Bio: {type: NODE-L2}\n").unwrap();
        let target = locate(dir.path(), &data, &rel(&["100.academic", "02.Bio"])).unwrap();
        assert!(target.ghost);
        assert!(!locate(dir.path(), &data, &rel(&["100.ACADEMIC", "02.Bio"])).unwrap().ghost);
    }

    #[test]
    fn mutations_refuse_while_another_process_holds_the_lock() {
        let dir = academic("locked-storage");
        let held = StorageLock::acquire(dir.path()).unwrap();
        let err = rename_node(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), "Chem").unwrap_err();
        assert!(matches!(err, FmError::Io { ref op, .. } if op == "lock storage"), "{err:?}");
        assert!(dir.path().join("100.ACADEMIC/02.Bio").is_dir());
        drop(held);
        rename_node(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), "Chem").unwrap();
    }

    #[test]
    fn a_tree_failure_after_commit_is_a_warning_not_an_error() {
        let dir = academic("after-commit");
        let failed = Err(FmError::Internal { message: "boom".to_string() });
        let m = after_commit(dir.path(), failed, Some(rel(&["x"])), vec!["w".to_string()]);
        assert!(m.tree.nodes.is_empty() && m.tree.yaml_changed);
        assert_eq!(m.select, Some(rel(&["x"])));
        assert_eq!(m.warnings.len(), 2);
        assert!(m.warnings[1].contains("refresh") && m.warnings[1].contains("boom"), "{:?}", m.warnings);
    }
}
