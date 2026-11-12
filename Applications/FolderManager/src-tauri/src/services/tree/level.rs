// The rows one folder shows, read without the storage lock by the IPC command and the MCP tools alike.
use std::path::Path;

use serde_yaml::Mapping;

use crate::domain::config::REVIEW_FOLDER;
use crate::domain::error::R;
use crate::infra::fs_safe::validate_component;
use crate::services::tree::tree_node::{build_children, fill_children, TreeNode};
use crate::services::tree::tree_store;

// The review folder lists links to owners anywhere in the tree; every other level needs only its own branch.
pub fn level(root: &Path, rel_parts: &[String]) -> R<(Mapping, Vec<TreeNode>)> {
    rel_parts.iter().try_for_each(|part| validate_component(part))?;
    let data = match rel_parts {
        [top] if top.eq_ignore_ascii_case(REVIEW_FOLDER) => tree_store::load(root)?,
        _ => tree_store::load_level(root, rel_parts)?,
    };
    let kids = build_children(root, &data, rel_parts)?;
    Ok((data, kids))
}

pub fn children(root: &Path, rel_parts: &[String]) -> R<Vec<TreeNode>> {
    let (data, mut kids) = level(root, rel_parts)?;
    fill_children(root, &data, &mut kids)?;
    Ok(kids)
}
