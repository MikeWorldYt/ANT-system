// What the read tools return: a short row per folder for listings, the full row with its owner for one node.
use serde::Serialize;

use crate::domain::config::{WkbStatus, OWNER_KEYS};
use crate::infra::storage::info::field_of;
use crate::services::tree::tree_node::TreeNode;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Brief {
    pub name: String,
    pub rel_parts: Vec<String>,
    #[serde(rename = "type")]
    pub node_type: Option<String>,
    pub status: Option<WkbStatus>,
    pub flags: Vec<String>,
    pub file: bool,
    pub ghost: bool,
    pub linked: bool,
    pub locked: bool,
    pub reserved: bool,
    pub favorite: bool,
    pub important: bool,
    pub hidden: bool,
    pub has_children: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Brief>>,
}

impl Brief {
    // `levels` counts this row, so 1 drops the children the tree already loaded.
    pub fn of(node: &TreeNode, levels: usize) -> Self {
        let children = node.children.as_ref().filter(|_| levels > 1).map(|kids| kids.iter().map(|k| Brief::of(k, levels - 1)).collect());
        Self {
            name: node.name.clone(),
            rel_parts: node.rel_parts.clone(),
            node_type: node.node_type.as_ref().map(|t| t.raw.clone()),
            status: node.status,
            flags: node.flags.clone(),
            file: node.file,
            ghost: node.ghost,
            linked: node.linked,
            locked: node.locked,
            reserved: node.reserved,
            favorite: node.favorite,
            important: node.important,
            hidden: node.hidden,
            has_children: node.has_children,
            children,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detail {
    #[serde(flatten)]
    pub node: TreeNode,
    pub owner: Option<String>,
}

impl Detail {
    pub fn of(node: TreeNode) -> Self {
        let owner = node.info.as_ref().and_then(|i| field_of(i, &OWNER_KEYS)).map(str::to_string);
        Self { node, owner }
    }
}
