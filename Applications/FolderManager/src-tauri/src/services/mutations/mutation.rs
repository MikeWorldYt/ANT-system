// The result every mutation returns: the levels the UI patches, and what the index needs to follow the change.
use serde::Serialize;
use serde_yaml::Mapping;

use super::LevelPatch;
use crate::services::tree::tree_node::{TreeNode, TreeSnapshot};
use crate::services::tree::tree_store::get_node;

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
            .filter_map(|rel| match &self.tree {
                Some(tree) => find_node(&tree.nodes, rel),
                None => {
                    let (_, parent) = rel.split_last()?;
                    let level = self.levels.iter().find(|l| l.parent == parent)?;
                    level.children.iter().find(|n| n.rel_parts == *rel)
                }
            })
            .collect()
    }
}

fn find_node<'a>(nodes: &'a [TreeNode], rel: &[String]) -> Option<&'a TreeNode> {
    let node = nodes.iter().find(|n| rel.starts_with(&n.rel_parts))?;
    if node.rel_parts == rel {
        return Some(node);
    }
    find_node(node.children.as_deref()?, rel)
}

// Clones only the given subtrees, so a mutation never keeps a copy of the whole old yaml.
pub(crate) fn subtrees(data: &Mapping, paths: &[Vec<String>]) -> Vec<(Vec<String>, Mapping)> {
    paths.iter().filter_map(|p| get_node(data, p).map(|node| (p.clone(), node.clone()))).collect()
}
