// Parses Tree.yaml and describes the folders it will create, so a caller can preview before touching disk.
use std::path::{Component, Path};

use serde_yaml::Mapping;

use crate::tree_types::{child_entries, node_type};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanNode {
    pub name: String,
    pub kind: String,
    pub children: Vec<PlanNode>,
}

pub fn parse_tree(tree_yaml: &str) -> Result<Mapping, String> {
    let value: serde_yaml::Value = serde_yaml::from_str(tree_yaml).map_err(|e| format!("Tree.yaml is not valid YAML: {e}"))?;
    let tree = value.as_mapping().ok_or("Tree.yaml is empty or not a mapping")?.clone();
    check_names(&tree)?;
    Ok(tree)
}

pub fn plan(tree_yaml: &str) -> Result<Vec<PlanNode>, String> {
    Ok(plan_nodes(&parse_tree(tree_yaml)?))
}

fn plan_nodes(m: &Mapping) -> Vec<PlanNode> {
    child_entries(m)
        .map(|(name, child)| PlanNode { name: name.to_string(), kind: node_type(child).to_string(), children: plan_nodes(child) })
        .collect()
}

// Each key becomes one folder name, so a key like "../x" must never leave the storage root.
fn check_names(m: &Mapping) -> Result<(), String> {
    for (name, child) in child_entries(m) {
        let mut parts = Path::new(name).components();
        let single = matches!((parts.next(), parts.next()), (Some(Component::Normal(_)), None));
        if !single || name.contains(['/', '\\', '\0']) {
            return Err(format!("Tree.yaml has an invalid folder name: {name:?}"));
        }
        check_names(child)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_keeps_order_kinds_and_nesting() {
        let nodes = plan("100.A:\n  type: NODE-L1\n  98.A:\n    type: REV-L2\n999.Settings:\n  type: CFG\n").unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].name, "100.A");
        assert_eq!(nodes[0].kind, "NODE");
        assert_eq!(nodes[0].children[0].kind, "REV");
        assert_eq!(nodes[1].kind, "CFG");
    }

    #[test]
    fn plan_rejects_non_mappings_and_escaping_names() {
        assert!(plan("").is_err());
        assert!(plan("- a\n- b\n").is_err());
        assert!(plan("\"../x\":\n  type: NODE\n").is_err());
        assert!(plan("a:\n  \"b/c\":\n    type: NODE\n").is_err());
    }
}
