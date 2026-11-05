// Reads the 998.Review layout: the links it lists, the owners' 98. folders and the filter kept in its info.json.
use std::fs;
use std::path::{Path, PathBuf};

use serde_yaml::{Mapping, Value};

use crate::domain::config::REVIEW_FOLDER;
use crate::domain::error::R;
use crate::domain::names::editable_part;
use crate::domain::shortcut_names::review_shortcut_name;
use crate::domain::tree_types::child_entries;
use crate::infra::fs_safe::{is_link, join_rel};
use crate::infra::storage::info::{field_of, read_info};
use crate::services::tree::depth;
use crate::services::tree::tree_node::is_reserved;
use crate::services::tree::tree_store::is_locked_type;

pub(crate) fn review_dir(root: &Path) -> Result<Option<PathBuf>, String> {
    let review_dir = root.join(REVIEW_FOLDER);
    match fs::symlink_metadata(&review_dir) {
        Ok(meta) if meta.is_dir() && !is_link(&meta) => Ok(Some(review_dir)),
        // Clearing .lnk files through a linked 998.Review would delete files outside the storage.
        Ok(meta) if is_link(&meta) => Err(format!("{REVIEW_FOLDER} is a link; index not rebuilt")),
        _ => Ok(None),
    }
}

pub(crate) fn rev_folder(root: &Path, owner: &[String]) -> PathBuf {
    let editable = owner.last().map_or("", |name| editable_part(name));
    join_rel(root, owner).join(format!("98.{editable}"))
}

// The links 998.Review lists: each existing shortcut with the owner it opens.
pub(crate) fn links(root: &Path, data: &Mapping) -> R<Vec<(String, Vec<String>)>> {
    let review = match review_dir(root) {
        Ok(Some(dir)) => dir,
        // A linked 998.Review is reported by the index rebuild, so the listing shows nothing.
        Ok(None) | Err(_) => return Ok(Vec::new()),
    };
    let limit = depth::limit(root)?;
    let mut found = Vec::new();
    owners(data, &[], &mut found, limit);
    let mut links = Vec::new();
    for owner in found {
        if is_reserved(&owner[..1]) {
            continue;
        }
        let link = review_shortcut_name(&owner, limit)?;
        if rev_folder(root, &owner).is_dir() && review.join(format!("{link}.lnk")).is_file() {
            links.push((link, owner));
        }
    }
    Ok(links)
}

// The owner folder a 998.Review link opens, so files dropped on the link land in its review folder.
pub(crate) fn linked_owner(root: &Path, data: &Mapping, rel: &[String]) -> R<Option<Vec<String>>> {
    let [_, name] = rel else { return Ok(None) };
    if !is_link_path(rel) {
        return Ok(None);
    }
    Ok(links(root, data)?.into_iter().find(|(link, _)| link == name).map(|(_, owner)| owner))
}

// Only a direct child of 998.Review can be a link.
pub(crate) fn is_link_path(rel: &[String]) -> bool {
    matches!(rel, [top, _] if top.eq_ignore_ascii_case(REVIEW_FOLDER))
}

// The review folder a link stands for, which is what opening it shows.
pub(crate) fn link_folder(root: &Path, data: &Mapping, rel: &[String]) -> R<Option<PathBuf>> {
    Ok(linked_owner(root, data, rel)?.map(|owner| rev_folder(root, &owner)))
}

// The review folder's storage path as the details show it, e.g. "200.WORK/03.Toki/98.Toki".
pub(crate) fn rev_path(owner: &[String]) -> String {
    let editable = owner.last().map_or("", |name| editable_part(name));
    let mut parts = owner.to_vec();
    parts.push(format!("98.{editable}"));
    parts.join("/")
}

// One field of the 998.Review filter, kept in that folder's info.json.
fn filter_field(root: &Path, key: &str) -> Option<String> {
    read_info(&root.join(REVIEW_FOLDER)).and_then(|info| field_of(&info, &[key]).map(str::to_string))
}

// The categories the user unchecked in the 998.Review filter.
pub(crate) fn hidden_categories(root: &Path) -> Vec<String> {
    filter_field(root, "hidden")
        .map(|text| text.split(',').map(str::trim).filter(|c| !c.is_empty()).map(str::to_string).collect())
        .unwrap_or_default()
}

// Starred links stay listed even when their category is hidden; an unchecked Favorites box turns that off.
pub(crate) fn favorites_shown(root: &Path) -> bool {
    filter_field(root, "favorites").as_deref() != Some("off")
}

pub(crate) fn is_locked(node: &Mapping) -> bool {
    is_locked_type(node.get("type").and_then(Value::as_str).unwrap_or(""))
}

// Sorted pre-order, so the full rebuild always hands its pairs over in the same order.
pub(crate) fn owners(node: &Mapping, parts: &[String], found: &mut Vec<Vec<String>>, limit: usize) {
    let mut children: Vec<(&str, &Mapping)> = child_entries(node).collect();
    children.sort_by(|a, b| a.0.cmp(b.0));
    for (name, child) in children {
        if is_locked(child) {
            continue;
        }
        let mut child_parts = parts.to_vec();
        child_parts.push(name.to_string());
        found.push(child_parts.clone());
        if child_parts.len() < limit {
            owners(child, &child_parts, found, limit);
        }
    }
}
