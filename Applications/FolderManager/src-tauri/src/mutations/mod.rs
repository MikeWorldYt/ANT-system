// Rename, retype and add nodes, keeping disk folders, linked 90./98. folders and the tree in step.
mod add;
mod copy_answer;
mod delete;
mod favorite;
mod file_drop;
mod move_folder;
mod user_flags;
mod recycle;
mod rename;
mod reorder;
mod result;
mod resolve;
mod retype;
#[cfg(not(windows))]
mod unix_trash;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_yaml::{Mapping, Value};

use crate::ant_core::tree_types::{child_entries, META_KEYS};
use crate::config::is_hidden_dir;
use crate::error::{FmError, R};
use crate::fs_safe::{io_err, resolve, same_name, Journal};
use crate::index::footprint::{self, Copies};
use crate::tree::tree_node::{is_reserved, TreeNode, TreeSnapshot};
use crate::tree::tree_store::{self, get_node, get_type, is_locked_type};

pub use add::{add_next, add_node};
pub use copy_answer::{answer_copy, CopyAnswer};
pub use delete::{delete_node, inspect_delete, DeletePreview};
pub use favorite::set_favorite;
pub use file_drop::{drop_files, DropOutcome};
pub(crate) use file_drop::free_name;
pub use move_folder::{move_folder, move_to_review};
pub use user_flags::set_user_flag;
pub use recycle::{recover_trashed, recycle_staged, PendingRecycle};
pub use rename::rename_node;
pub use reorder::{move_node, MoveDirection};
pub use result::LevelPatch;
pub(crate) use result::finish;
pub use resolve::{resolve_invalid, Resolution};
pub use retype::{set_details, set_node_type};
#[cfg(test)]
pub(crate) use recycle::recycle_staged_with;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mutation {
    // Only set when the change is not local (a re-synced yaml, an unknown scope or a failed patch).
    pub tree: Option<TreeSnapshot>,
    pub levels: Vec<LevelPatch>,
    pub select: Option<Vec<String>>,
    pub warnings: Vec<String>,
    // Subtrees whose search docs changed, before and after; empty means unknown and forces a full rebuild.
    #[serde(skip)]
    pub touched: Vec<Vec<String>>,
    // Pre-mutation yaml of the touched paths that existed, so data derived from removed paths can be found.
    #[serde(skip)]
    pub before: Vec<(Vec<String>, Mapping)>,
}

impl Mutation {
    pub(crate) fn replacing(mut self, before: Vec<(Vec<String>, Mapping)>) -> Self {
        self.before = before;
        self
    }

    pub(crate) fn touched_nodes(&self) -> Vec<&TreeNode> {
        self.touched
            .iter()
            .filter_map(|rel| {
                let (_, parent) = rel.split_last()?;
                let level = self.levels.iter().find(|l| l.parent == parent)?;
                level.children.iter().find(|n| n.rel_parts == *rel)
            })
            .collect()
    }
}

// Clones only the given subtrees, so a mutation never keeps a copy of the whole old yaml.
pub(crate) fn subtrees(data: &Mapping, paths: &[Vec<String>]) -> Vec<(Vec<String>, Mapping)> {
    paths.iter().filter_map(|p| get_node(data, p).map(|node| (p.clone(), node.clone()))).collect()
}

pub(crate) struct Target {
    pub abs: PathBuf,
    pub ghost: bool,
    pub node_type: String,
}

// New .ant folders get their id now; true means a folder moved outside the app was re-keyed, which is not a local change.
fn register_ids(root: &Path, data: &mut Mapping, warnings: &mut Vec<String>) -> bool {
    let shape = node_paths(data);
    match footprint::plan(root, data, Copies::Ignore).map(|plan| plan.changed) {
        Ok(true) => {
            if let Err(e) = tree_store::save(root, data) {
                warnings.push(format!("Could not save moved folders: {e}"));
            }
            node_paths(data) != shape
        }
        Ok(false) => false,
        Err(e) => {
            warnings.push(format!("Could not assign folder ids: {e}"));
            false
        }
    }
}

fn node_paths(data: &Mapping) -> BTreeSet<Vec<String>> {
    fn walk(node: &Mapping, rel: &mut Vec<String>, out: &mut BTreeSet<Vec<String>>) {
        for (name, child) in child_entries(node) {
            rel.push(name.to_string());
            out.insert(rel.clone());
            walk(child, rel, out);
            rel.pop();
        }
    }
    let mut out = BTreeSet::new();
    walk(data, &mut Vec::new(), &mut out);
    out
}

// Folders added on disk since the last refresh get their yaml entry before any check runs.
pub(crate) fn load_synced(root: &Path) -> R<(Mapping, bool)> {
    let mut data = tree_store::load(root)?;
    let changed = tree_store::sync(root, &mut data)?;
    Ok((data, changed))
}

// Ordinary mutations list only the levels down to `dir`; drift found there re-syncs everything, as the change is then not local.
pub(crate) fn load_synced_at(root: &Path, dir: &[String]) -> R<(Mapping, bool)> {
    let mut data = tree_store::load(root)?;
    let changed = tree_store::sync_branch(root, &mut data, dir)?;
    if changed {
        tree_store::sync(root, &mut data)?;
    }
    Ok((data, changed))
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
    Ok(Target { abs, ghost: !on_disk, node_type: get_type(data, rel) })
}

fn exactly_on_disk(root: &Path, rel: &[String]) -> R<bool> {
    let mut dir = root.to_path_buf();
    for part in rel {
        if !has_exact_dir(&dir, part)? {
            return Ok(false);
        }
        dir.push(part);
    }
    Ok(true)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{EditableType, NewKind};
    use crate::fs_safe::StorageLock;
    use crate::test_support::*;

    #[test]
    fn meta_key_folders_are_refused_and_the_parent_type_survives() {
        let dir = academic("meta-key");
        dir.mkdir("100.ACADEMIC/type");
        dir.mkdir("100.ACADEMIC/01.Math/status");
        let refused = |r: R<Mutation>| matches!(r, Err(FmError::InvalidName { .. }));
        assert!(refused(rename_node(dir.path(), &rel(&["100.ACADEMIC", "type"]), "X")));
        assert!(refused(set_node_type(dir.path(), &rel(&["100.ACADEMIC", "type"]), EditableType::Wkb, None)));
        assert!(refused(add_node(dir.path(), &rel(&["100.ACADEMIC", "01.Math", "status"]), NewKind::Node)));
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
}
