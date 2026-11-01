// Describes the folders the template will create, so a caller can preview before touching disk.
use std::path::{Component, Path};

use serde_yaml::Mapping;

use crate::template::tree_from_template;
use crate::tree_types::{child_entries, node_type};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanNode {
    pub name: String,
    pub kind: String,
    pub children: Vec<PlanNode>,
}

pub fn plan(template: &[u8]) -> Result<Vec<PlanNode>, String> {
    Ok(plan_nodes(&tree_from_template(template)?))
}

fn plan_nodes(m: &Mapping) -> Vec<PlanNode> {
    child_entries(m)
        .map(|(name, child)| PlanNode { name: name.to_string(), kind: node_type(child).to_string(), children: plan_nodes(child) })
        .collect()
}

// Each key becomes one folder name, so a key like "../x" must never leave the storage root.
pub(crate) fn check_names(m: &Mapping) -> Result<(), String> {
    for (name, child) in child_entries(m) {
        let mut parts = Path::new(name).components();
        let single = matches!((parts.next(), parts.next()), (Some(Component::Normal(_)), None));
        if !single || name.contains(['/', '\\', '\0']) {
            return Err(format!("The template has an invalid folder name: {name:?}"));
        }
        check_names(child)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TEMPLATE;

    #[test]
    fn plan_keeps_order_kinds_and_nesting() {
        let nodes = plan(TEMPLATE).unwrap();
        assert_eq!(nodes[0].name, "100.PERSONAL");
        assert_eq!(nodes[0].kind, "NODE");
        assert_eq!(nodes[0].children[0].name, "01.Documents");
        assert_eq!(nodes[0].children[0].children[0].kind, "REV");
    }

    #[test]
    fn plan_rejects_files_that_are_not_templates() {
        assert!(plan(b"").is_err());
        assert!(plan(b"- a\n- b\n").is_err());
    }

    #[test]
    fn names_that_escape_the_root_are_refused() {
        let escaping: Mapping = serde_yaml::from_str("\"../x\":\n  type: NODE\n").unwrap();
        let nested: Mapping = serde_yaml::from_str("a:\n  \"b/c\":\n    type: NODE\n").unwrap();
        assert!(check_names(&escaping).is_err());
        assert!(check_names(&nested).is_err());
    }
}
