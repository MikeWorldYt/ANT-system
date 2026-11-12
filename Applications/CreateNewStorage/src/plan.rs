// Describes the folders the template will create, so a caller can preview before touching disk.
use std::path::{Component, Path};

use crate::template::tree_from_template;
use crate::tree_types::TreeNode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanNode {
    pub name: String,
    pub kind: String,
    pub children: Vec<PlanNode>,
}

pub fn plan(template: &[u8]) -> Result<Vec<PlanNode>, String> {
    Ok(plan_nodes(&tree_from_template(template)?))
}

fn plan_nodes(nodes: &[TreeNode]) -> Vec<PlanNode> {
    nodes
        .iter()
        .map(|n| PlanNode { name: n.name.clone(), kind: n.base_kind().to_string(), children: plan_nodes(&n.children) })
        .collect()
}

// Each node becomes one folder name, so a name like "../x" must never leave the storage root.
pub(crate) fn check_names(nodes: &[TreeNode]) -> Result<(), String> {
    for node in nodes {
        let name = node.name.as_str();
        let mut parts = Path::new(name).components();
        let single = matches!((parts.next(), parts.next()), (Some(Component::Normal(_)), None));
        if !single || name.contains(['/', '\\', '\0']) {
            return Err(format!("The template has an invalid folder name: {name:?}"));
        }
        check_names(&node.children)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{node, TEMPLATE};

    #[test]
    fn plan_keeps_order_kinds_and_nesting() {
        let nodes = plan(TEMPLATE).unwrap();
        assert_eq!(nodes[0].name, "100.PERSONAL");
        assert_eq!(nodes[0].kind, "NODE");
        assert_eq!(nodes[0].children[0].name, "01.Documents");
        assert_eq!(nodes[0].children[0].children.last().unwrap().kind, "REV");
    }

    #[test]
    fn plan_rejects_files_that_are_not_templates() {
        assert!(plan(b"").is_err());
        assert!(plan(b"- a\n- b\n").is_err());
    }

    #[test]
    fn names_that_escape_the_root_are_refused() {
        let escaping = vec![node("../x", "NODE", vec![])];
        let nested = vec![node("a", "NODE", vec![node("b/c", "NODE", vec![])])];
        assert!(check_names(&escaping).is_err());
        assert!(check_names(&nested).is_err());
    }
}
