// Tree rows sent to the UI: disk folders merged with tree entries, plus the per-node action rules.
mod actions;
mod entries;
#[cfg(test)]
mod tests;

pub use actions::NodeActions;

use std::path::Path;

use serde::Serialize;
use serde_yaml::{Mapping, Value};

use crate::domain::config::{is_hidden_dir, TypeBase, WkbStatus, MAIN_FLAG, REVIEW_FOLDER, SETTINGS_FOLDER};
use crate::domain::error::R;
use crate::domain::model::Copied;
use crate::domain::names::{is_system_root_name, is_workboard_name, name_parts};
use crate::domain::tree_types::{flags_in, type_base, type_level, META_KEYS};
use crate::infra::fs_safe::join_rel;
use crate::infra::scan::list_subdirs;
use crate::infra::storage::info::{field_of, read_info, NodeInfo};
use crate::services::tree::depth;
use crate::services::tree::review_links;
use crate::services::tree::tree_store::{get_node, get_type, is_locked_type};
use actions::{compute_actions, file_actions, link_actions, mark_moves};
use entries::{is_unmanaged, level_entries, Entry};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeType {
    pub raw: String,
    pub base: TypeBase,
    pub level: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeNode {
    pub name: String,
    pub rel_parts: Vec<String>,
    pub depth: usize,
    pub ghost: bool,
    pub in_yaml: bool,
    pub node_type: Option<NodeType>,
    pub status: Option<WkbStatus>,
    pub status_raw: Option<String>,
    pub locked: bool,
    pub reserved: bool,
    pub linked: bool,
    pub link_path: Option<String>,
    pub flags: Vec<String>,
    pub has_children: bool,
    pub is_git: bool,
    pub fixed_prefix: String,
    pub editable_name: String,
    pub favorite: bool,
    pub important: bool,
    pub hidden: bool,
    pub file: bool,
    pub info: Option<NodeInfo>,
    pub children: Option<Vec<TreeNode>>,
    pub abs_path: String,
    pub actions: NodeActions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeSnapshot {
    pub root: String,
    pub nodes: Vec<TreeNode>,
    pub yaml_changed: bool,
    pub copies: Vec<Copied>,
    pub warnings: Vec<String>,
}

pub fn build_tree(root: &Path, data: &Mapping, yaml_changed: bool) -> R<TreeSnapshot> {
    Ok(TreeSnapshot {
        root: root.to_string_lossy().into_owned(),
        nodes: children_at(root, data, &[], depth::limit(root)?)?,
        yaml_changed,
        copies: Vec::new(),
        warnings: Vec::new(),
    })
}

pub fn build_children(root: &Path, data: &Mapping, parent: &[String]) -> R<Vec<TreeNode>> {
    children_at(root, data, parent, depth::limit(root)?)
}

// Every folder a level holds, hidden ones included, for the manage list that decides what is hidden.
pub(crate) fn managed_children(root: &Path, data: &Mapping, parent: &[String]) -> R<Vec<TreeNode>> {
    let dir = join_rel(root, parent);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let limit = depth::limit(root)?;
    let mut nodes = Vec::new();
    for name in list_subdirs(&dir)?.into_iter().filter(|n| !META_KEYS.contains(&n.as_str()) && !is_hidden_dir(n)) {
        let mut rel = parent.to_vec();
        rel.push(name.clone());
        let in_yaml = get_node(data, &rel).is_some();
        let entry = Entry { name, on_disk: true, in_yaml, link: None, favorite: false, hidden: false, file: false };
        nodes.push(make_node(root, data, parent, entry, limit)?);
    }
    Ok(nodes)
}

pub fn fill_children(root: &Path, data: &Mapping, nodes: &mut [TreeNode]) -> R<()> {
    fill_at(root, data, nodes, depth::limit(root)?)
}

fn children_at(root: &Path, data: &Mapping, parent: &[String], limit: usize) -> R<Vec<TreeNode>> {
    let mut nodes = Vec::new();
    // A review folder at the depth limit shows its files and the folders the database does not track, so they can be browsed.
    let past_limit_review = parent.len() >= limit && holds_files(data, parent);
    for entry in level_entries(root, data, parent)? {
        if entry.file {
            nodes.push(file_node(root, parent, entry.name));
        } else if !past_limit_review || is_unmanaged(&entry) {
            nodes.push(make_node(root, data, parent, entry, limit)?);
        }
    }
    mark_moves(&mut nodes);
    Ok(nodes)
}

// Below the limit the children load when their folder opens, so the disk is not walked to the bottom.
fn fill_at(root: &Path, data: &Mapping, nodes: &mut [TreeNode], limit: usize) -> R<()> {
    for node in nodes {
        if node.has_children && node.depth + 1 < limit {
            let mut kids = children_at(root, data, &node.rel_parts, limit)?;
            fill_at(root, data, &mut kids, limit)?;
            node.children = Some(kids);
        }
    }
    Ok(())
}

// Any of these inside a folder means it holds a git project.
const GIT_MARKERS: [&str; 3] = [".git", ".github", ".gitignore"];

// Review folders and workboards hold files, so they open even when empty.
fn holds_files(data: &Mapping, rel: &[String]) -> bool {
    matches!(type_base(&get_type(data, rel)), "REV" | "WKB")
}

// Every folder lists the files inside it, except the app's own reserved folders and what they hold.
pub(crate) fn lists_files(rel: &[String]) -> bool {
    !rel.first().is_some_and(|top| is_reserved(std::slice::from_ref(top)))
}

// Files are read from disk when their folder opens; they carry no info, children or actions.
fn file_node(root: &Path, parent: &[String], name: String) -> TreeNode {
    let mut rel_parts = parent.to_vec();
    rel_parts.push(name.clone());
    TreeNode {
        abs_path: join_rel(root, &rel_parts).to_string_lossy().into_owned(),
        rel_parts,
        depth: parent.len(),
        ghost: false,
        in_yaml: false,
        node_type: None,
        status: None,
        status_raw: None,
        locked: false,
        reserved: false,
        linked: false,
        link_path: None,
        flags: Vec::new(),
        has_children: false,
        is_git: false,
        fixed_prefix: String::new(),
        editable_name: name.clone(),
        favorite: false,
        important: false,
        hidden: false,
        file: true,
        info: None,
        children: None,
        actions: file_actions(),
        name,
    }
}

fn make_node(root: &Path, data: &Mapping, parent: &[String], entry: Entry, limit: usize) -> R<TreeNode> {
    let mut rel_parts = parent.to_vec();
    rel_parts.push(entry.name.clone());
    let depth = parent.len();
    let ghost = !entry.on_disk;
    let linked = entry.link.is_some();

    let yaml_entry = get_node(data, &rel_parts);
    // The tree only indexes down to the depth limit; a workboard-named folder it does not hold is still a workboard.
    let untracked_wkb = yaml_entry.is_none() && entry.on_disk && entry.link.is_none() && is_workboard_name(&entry.name);
    let raw_type = if untracked_wkb { "WKB" } else { yaml_entry.and_then(|m| m.get("type")).and_then(Value::as_str).unwrap_or("") };
    // A review link stands for its owner's review folder, so that is where its path and drops point.
    let abs_path = match &entry.link {
        Some(owner) => review_links::rev_folder(root, owner),
        None => join_rel(root, &rel_parts),
    };
    let info = if ghost || linked { None } else { read_info(&abs_path) };
    // A status in the folder's own info.json is the one shown; the tree one is the fallback.
    let status_raw = info
        .iter()
        .flat_map(|i| &i.fields)
        .find(|f| f.key.eq_ignore_ascii_case("status"))
        .map(|f| f.value.clone())
        .or_else(|| yaml_entry.and_then(|m| m.get("status")).and_then(Value::as_str).map(str::to_string));
    let node_type = (!raw_type.is_empty()).then(|| parse_type(raw_type));
    let is_wkb = node_type.as_ref().is_some_and(|t| t.base == TypeBase::Wkb);
    let status = status_raw.as_deref().filter(|_| is_wkb).and_then(parse_status);

    let locked = is_locked_type(raw_type);
    let reserved = is_reserved(&rel_parts);
    // Every real folder opens, even when empty or past the depth limit, where its children are read from disk on demand.
    let has_children = !ghost && !linked;
    let is_git = !ghost && !linked && GIT_MARKERS.iter().any(|m| abs_path.join(m).exists());

    let info_broken = info.as_ref().is_some_and(|i| i.error.is_some());

    let (fixed_prefix, editable_name) = name_parts(&entry.name, is_wkb);
    let mut actions = if linked { link_actions() } else { compute_actions(depth, limit, ghost, raw_type, reserved, info_broken) };
    if untracked_wkb {
        // These edit the tree entry, which an untracked folder does not have.
        actions.change_type = false;
        actions.add_next = false;
        actions.add_child = false;
        actions.edit_flags = false;
        actions.hide = false;
    }
    Ok(TreeNode {
        actions,
        name: entry.name,
        rel_parts,
        depth,
        ghost,
        in_yaml: entry.in_yaml,
        node_type,
        status,
        status_raw,
        locked,
        reserved,
        linked,
        link_path: entry.link.as_deref().map(|owner| review_links::rev_path(root, owner)),
        flags: node_flags(depth, yaml_entry),
        has_children,
        is_git,
        fixed_prefix,
        editable_name,
        favorite: entry.favorite || info.as_ref().and_then(|i| field_of(i, &["favorite"])).is_some_and(|v| v.eq_ignore_ascii_case("true")),
        important: is_wkb && info.as_ref().and_then(|i| field_of(i, &["important"])).is_some_and(|v| v.eq_ignore_ascii_case("true")),
        hidden: entry.hidden,
        file: false,
        info,
        children: None,
        abs_path: abs_path.to_string_lossy().into_owned(),
    })
}

// Root folders are always main, a rule rather than a stored flag; the rest carry what the user set.
fn node_flags(depth: usize, yaml_entry: Option<&Mapping>) -> Vec<String> {
    let mut flags = if depth == 0 { vec![MAIN_FLAG.to_string()] } else { Vec::new() };
    flags.extend(yaml_entry.map(flags_in).unwrap_or_default());
    flags
}

// Windows folder names ignore case, so "999.settings" is the same reserved folder; the whole 9xx range is the app's.
pub(crate) fn is_reserved(rel_parts: &[String]) -> bool {
    matches!(rel_parts, [name]
        if name.eq_ignore_ascii_case(SETTINGS_FOLDER) || name.eq_ignore_ascii_case(REVIEW_FOLDER) || is_system_root_name(name))
}

fn parse_type(raw: &str) -> NodeType {
    let base = TypeBase::from_str(type_base(raw)).unwrap_or(TypeBase::Unknown);
    let level = type_level(raw).and_then(|l| l.strip_prefix('L')).and_then(|n| n.parse().ok());
    NodeType { raw: raw.to_string(), base, level }
}

pub(crate) fn parse_status(raw: &str) -> Option<WkbStatus> {
    WkbStatus::from_str(&raw.trim().to_lowercase().replace(['_', ' '], "-"))
}
