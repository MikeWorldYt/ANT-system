// Hidden folders keep their place in the storage; these mutations set the flag and list the folders a level holds.
use std::path::Path;

use serde::Serialize;
use serde_yaml::Mapping;

use super::{child_rel, finish, guard_frozen, load_synced_at, locate, parent_of, Mutation};
use crate::ant_core::names::list_subdirs;
use crate::ant_core::tree_types::{flags_in, META_KEYS};
use crate::config::{is_hidden_dir, HIDDEN_FLAG};
use crate::error::{FmError, R};
use crate::fs_safe::StorageLock;
use crate::tree::tree_node::{is_reserved, managed_children, TreeNode};
use crate::tree::tree_store::{self, get_node_mut, get_type, is_locked_type};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HiddenChoice {
    pub node: TreeNode,
    pub hidden: bool,
    pub fixed: bool,
}

pub fn set_hidden(root: &Path, rel: &[String], on: bool) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, _) = load_synced_at(root, parent_of(rel))?;
    let target = locate(root, &data, rel)?;
    guard_frozen(rel, &target.node_type)?;
    if target.ghost {
        return Err(FmError::GhostNotAllowed { op: "hide".to_string() });
    }
    if rel.len() < 2 {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "root folders cannot be hidden".to_string() });
    }
    let node = get_node_mut(&mut data, rel).ok_or_else(|| FmError::NotFound { rel_parts: rel.to_vec() })?;
    write_flag(node, on);
    tree_store::save(root, &data)?;
    Ok(finish(root, &data, true, parent_selection(rel), Vec::new(), vec![rel.to_vec()]))
}

// `hidden` names the children of `parent` that stay hidden; the others are shown again. Reserved and locked folders are left as they are.
pub fn set_hidden_children(root: &Path, parent: &[String], hidden: &[String]) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, _) = load_synced_at(root, parent)?;
    let dir = if parent.is_empty() {
        root.to_path_buf()
    } else {
        let target = locate(root, &data, parent)?;
        guard_frozen(parent, &target.node_type)?;
        target.abs
    };
    for name in list_subdirs(&dir)?.into_iter().filter(|n| listable(n)) {
        let rel = child_rel(parent, &name);
        if is_fixed(&data, &rel) {
            continue;
        }
        if let Some(node) = get_node_mut(&mut data, &rel) {
            write_flag(node, hidden.contains(&name));
        }
    }
    tree_store::save(root, &data)?;
    Ok(finish(root, &data, true, parent_selection(parent), Vec::new(), vec![parent.to_vec()]))
}

// Every folder a level holds, with whether it is hidden, so the manage dialog can show them all.
pub fn hidden_choices(root: &Path, parent: &[String]) -> R<Vec<HiddenChoice>> {
    let data = tree_store::load(root)?;
    Ok(managed_children(root, &data, parent)?
        .into_iter()
        .map(|node| {
            let hidden = node.flags.iter().any(|f| f == HIDDEN_FLAG);
            let fixed = is_fixed(&data, &node.rel_parts);
            HiddenChoice { node, hidden, fixed }
        })
        .collect())
}

fn listable(name: &str) -> bool {
    !META_KEYS.contains(&name) && !is_hidden_dir(name)
}

fn is_fixed(data: &Mapping, rel: &[String]) -> bool {
    is_reserved(rel) || is_locked_type(&get_type(data, rel))
}

fn write_flag(node: &mut Mapping, on: bool) {
    let mut flags = flags_in(node);
    flags.retain(|f| f != HIDDEN_FLAG);
    if on {
        flags.push(HIDDEN_FLAG.to_string());
    }
    if flags.is_empty() {
        node.remove("flags");
    } else {
        node.insert("flags".into(), flags.join(",").into());
    }
}

// After a hide the view stays on the folder the hidden one sat in, since the hidden folder is no longer listed.
fn parent_selection(rel: &[String]) -> Option<Vec<String>> {
    let parent = parent_of(rel);
    (!parent.is_empty()).then(|| parent.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::footprint::Copies;
    use crate::test_support::*;

    fn listed_under_root_child(dir: &Path) -> Vec<String> {
        let tree = crate::tree::snapshot(dir, Copies::Tail).unwrap();
        tree.nodes[0].children.as_ref().unwrap().iter().map(|n| n.name.clone()).collect()
    }

    #[test]
    fn a_hidden_folder_leaves_the_tree_and_comes_back_when_shown() {
        let dir = academic("hidden-node");
        let bio = rel(&["100.ACADEMIC", "02.Bio"]);
        set_hidden(dir.path(), &bio, true).unwrap();
        assert!(!listed_under_root_child(dir.path()).contains(&"02.Bio".to_string()));
        assert!(dir.path().join("100.ACADEMIC/02.Bio").is_dir());
        set_hidden(dir.path(), &bio, false).unwrap();
        assert!(listed_under_root_child(dir.path()).contains(&"02.Bio".to_string()));
    }

    #[test]
    fn the_manage_list_marks_hidden_folders_and_leaves_reserved_ones_alone() {
        let dir = academic("hidden-choices");
        set_hidden_children(dir.path(), &rel(&["100.ACADEMIC"]), &["02.Bio".to_string()]).unwrap();
        let choices = hidden_choices(dir.path(), &rel(&["100.ACADEMIC"])).unwrap();
        let bio = choices.iter().find(|c| c.node.name == "02.Bio").unwrap();
        assert!(bio.hidden && !bio.fixed);
        let math = choices.iter().find(|c| c.node.name == "01.Math").unwrap();
        assert!(!math.hidden);
        set_hidden_children(dir.path(), &rel(&["100.ACADEMIC"]), &[]).unwrap();
        assert!(hidden_choices(dir.path(), &rel(&["100.ACADEMIC"])).unwrap().iter().all(|c| !c.hidden));
    }
}
