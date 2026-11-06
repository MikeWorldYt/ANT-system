// Lists the numeric support folders inside the tree's depth limit; deeper folders are the user's own content.
use std::path::Path;

use super::SupportFolder;
use crate::domain::config::SUPPORT_NODES;
use crate::domain::error::R;
use crate::domain::names::{editable_part, has_node_id, is_legacy_support, is_system_root_name, next_available_id};
use crate::infra::fs_safe::join_rel;
use crate::infra::scan::list_subdirs;
use crate::services::tree::depth;

// Other programs may point at resource folders, so these wait for the user instead of renaming on their own.
const RESOURCES_LEGACY: &str = "90.";

pub(super) fn legacy_folders(root: &Path) -> R<Vec<SupportFolder>> {
    let limit = depth::limit(root)?;
    let mut found = Vec::new();
    for top in list_subdirs(root)? {
        if !is_system_root_name(&top) {
            walk(root, &[top], limit, &mut found)?;
        }
    }
    Ok(found)
}

// Only id-named nodes hold support folders, so the walk stays out of workboards and loose content.
fn walk(root: &Path, parent: &[String], limit: usize, found: &mut Vec<SupportFolder>) -> R<()> {
    if parent.len() >= limit {
        return Ok(());
    }
    for name in list_subdirs(&join_rel(root, parent))? {
        let mut rel = parent.to_vec();
        rel.push(name.clone());
        if is_legacy_support(&name) {
            found.push(SupportFolder { notice: name.starts_with(RESOURCES_LEGACY), proposed: proposed_name(root, parent, &name)?, rel_parts: rel, name });
        } else if has_node_id(&name) {
            walk(root, &rel, limit, found)?;
        }
    }
    Ok(())
}

fn proposed_name(root: &Path, parent: &[String], name: &str) -> R<String> {
    let editable = editable_part(name);
    if is_support(parent.last().map_or("", String::as_str), name) {
        let word = SUPPORT_NODES.iter().find(|(_, legacy, _)| name.starts_with(legacy)).map_or("", |(word, _, _)| word);
        return Ok(format!("{word}{editable}"));
    }
    Ok(format!("{:02}.{editable}", next_available_id(&list_subdirs(&join_rel(root, parent))?)))
}

// The name alone tells a support folder: a review repeats its parent's name, templates and resources say what they hold.
pub(super) fn is_support(parent: &str, name: &str) -> bool {
    let editable = editable_part(name);
    match name.get(..3) {
        Some("98.") => editable.eq_ignore_ascii_case(editable_part(parent)),
        Some("97.") => editable.to_lowercase().contains("template"),
        Some(RESOURCES_LEGACY) => editable.to_lowercase().contains("resource"),
        _ => false,
    }
}
