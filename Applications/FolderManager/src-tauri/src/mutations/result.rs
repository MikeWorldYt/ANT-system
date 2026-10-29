// Builds the Mutation a change returns: parent-level patches when local, else a full snapshot.
use std::path::Path;

use serde::Serialize;
use serde_yaml::Mapping;

use super::{exactly_on_disk, split, Mutation};
use crate::error::R;
use crate::tree::depth;
use crate::tree::tree_node::{build_children, build_tree, fill_children, TreeNode, TreeSnapshot};
use crate::tree::tree_store::{self, get_node};

// One parent's children as they are now; touched children carry their filled subtree, the others none.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelPatch {
    pub parent: Vec<String>,
    pub has_children: bool,
    pub children: Vec<TreeNode>,
}

// Never fails, as the change is already on disk; a local change sends only the touched paths' parent levels.
pub(crate) fn finish(
    root: &Path,
    data: &Mapping,
    yaml_changed: bool,
    select: Option<Vec<String>>,
    warnings: Vec<String>,
    touched: Vec<Vec<String>>,
) -> Mutation {
    let local = !yaml_changed && !touched.is_empty();
    match local.then(|| subtree_drift(root, data, &touched)) {
        Some(Ok(false)) => match patch_levels(root, data, &touched) {
            Ok(levels) => return Mutation { tree: None, levels, select, warnings, touched, before: Vec::new() },
            Err(e) => return after_commit(root, Err(e), select, warnings, touched),
        },
        Some(Err(e)) => return after_commit(root, Err(e), select, warnings, touched),
        _ => {}
    }
    // Reaching here with a local scope means drift was found inside a touched subtree.
    let tree = build_tree(root, data, yaml_changed || local).and_then(|mut tree| {
        fill_children(root, data, &mut tree.nodes)?;
        Ok(tree)
    });
    after_commit(root, tree, select, warnings, touched)
}

// The quick sync stops above a touched subtree, so folders added deeper inside it would reach the UI unsynced.
fn subtree_drift(root: &Path, data: &Mapping, touched: &[Vec<String>]) -> R<bool> {
    for rel in touched {
        if let Some(node) = get_node(data, rel) {
            if tree_store::sync_subtree(root, rel, &mut node.clone())? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

// Mirrors the snapshot's rules: a ghost parent or one on the last level shows no children.
fn patch_levels(root: &Path, data: &Mapping, touched: &[Vec<String>]) -> R<Vec<LevelPatch>> {
    let limit = depth::limit(root)?;
    let mut levels: Vec<LevelPatch> = Vec::new();
    for rel in touched {
        let (_, parent) = split(rel)?;
        if levels.iter().any(|l| l.parent == parent) {
            continue;
        }
        let mut children = build_children(root, data, parent)?;
        let has_children = parent.is_empty() || (parent.len() < limit && !children.is_empty() && exactly_on_disk(root, parent)?);
        if !has_children {
            children.clear();
        }
        for child in children.iter_mut().filter(|n| touched.contains(&n.rel_parts)) {
            fill_children(root, data, std::slice::from_mut(child))?;
        }
        levels.push(LevelPatch { parent: parent.to_vec(), has_children, children });
    }
    Ok(levels)
}

// An empty tree with yamlChanged set tells the UI to refresh instead of showing a stale tree.
pub(super) fn after_commit(
    root: &Path,
    tree: R<TreeSnapshot>,
    select: Option<Vec<String>>,
    mut warnings: Vec<String>,
    touched: Vec<Vec<String>>,
) -> Mutation {
    let tree = tree.unwrap_or_else(|e| {
        warnings.push(format!("The change was saved but the tree could not be reloaded; refresh. {e}"));
        TreeSnapshot { root: root.to_string_lossy().into_owned(), nodes: Vec::new(), yaml_changed: true, copies: Vec::new() }
    });
    Mutation { tree: Some(tree), levels: Vec::new(), select, warnings, touched, before: Vec::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::FmError;
    use crate::mutations::rename_node;
    use crate::test_support::*;

    #[test]
    fn a_local_mutation_returns_only_the_parent_level_with_the_touched_subtree_filled() {
        let dir = academic("finish-depth");
        let mut data = yaml(dir.path());
        tree_store::sync(dir.path(), &mut data).unwrap();
        tree_store::save(dir.path(), &data).unwrap();
        let m = rename_node(dir.path(), &rel(&["100.ACADEMIC", "01.Math"]), "Calc").unwrap();
        assert!(m.tree.is_none());
        assert_eq!(m.levels.len(), 1);
        let level = &m.levels[0];
        assert_eq!(level.parent, rel(&["100.ACADEMIC"]));
        assert!(level.has_children);
        let calc = level.children.iter().find(|n| n.name == "01.Calc").unwrap();
        assert!(calc.children.as_ref().is_some_and(|k| k.iter().any(|n| n.name == "01.Algebra")), "{calc:?}");
        assert!(level.children.iter().find(|n| n.name == "02.Bio").is_some_and(|n| n.children.is_none()));
        assert_eq!(m.touched_nodes().len(), 1);
    }

    #[test]
    fn drift_found_while_syncing_returns_the_full_snapshot() {
        let dir = academic("finish-drift");
        dir.mkdir("100.ACADEMIC/05.Fresh");
        dir.mkdir("200.ELSEWHERE/01.Deep");
        let m = rename_node(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), "Chem").unwrap();
        let tree = m.tree.expect("a re-synced yaml is not a local change");
        assert!(tree.yaml_changed && m.levels.is_empty());
        let academic = tree.nodes.iter().find(|n| n.name == "100.ACADEMIC").and_then(|n| n.children.clone()).unwrap();
        assert!(academic.iter().any(|n| n.name == "02.Chem") && academic.iter().any(|n| n.name == "05.Fresh"));
        assert_eq!(yaml_type(dir.path(), &["200.ELSEWHERE", "01.Deep"]), "NODE-L2");
    }

    #[test]
    fn a_tree_failure_after_commit_is_a_warning_not_an_error() {
        let dir = academic("after-commit");
        let failed = Err(FmError::Internal { message: "boom".to_string() });
        let m = after_commit(dir.path(), failed, Some(rel(&["x"])), vec!["w".to_string()], Vec::new());
        let tree = m.tree.as_ref().unwrap();
        assert!(tree.nodes.is_empty() && tree.yaml_changed);
        assert_eq!(m.select, Some(rel(&["x"])));
        assert_eq!(m.warnings.len(), 2);
        assert!(m.warnings[1].contains("refresh") && m.warnings[1].contains("boom"), "{:?}", m.warnings);
    }
}
