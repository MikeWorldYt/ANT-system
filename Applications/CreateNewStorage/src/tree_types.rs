// The folder tree a template describes; stored node types carry a level suffix ("NODE-L2", "REV-L3", "CFG").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeNode {
    pub name: String,
    pub kind: String,
    pub fields: Vec<(String, String)>,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    pub fn base_kind(&self) -> &str {
        self.kind.split('-').next().unwrap_or("")
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::node;

    #[test]
    fn base_kind_drops_the_level_suffix() {
        assert_eq!(node("a", "NODE-L2", vec![]).base_kind(), "NODE");
        assert_eq!(node("a", "CFG", vec![]).base_kind(), "CFG");
    }
}
