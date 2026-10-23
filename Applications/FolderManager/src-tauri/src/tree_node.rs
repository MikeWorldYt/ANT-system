// Tree rows sent to the UI: disk folders merged with tree entries, plus the per-node action rules.
use std::collections::BTreeSet;
use std::path::Path;

use serde::Serialize;
use serde_yaml::{Mapping, Value};

use crate::ant_core::tree_types::{child_entries, type_base, type_level, META_KEYS};
use crate::config::{
    LINKED_PREFIXES, REVIEW_FOLDER, SETTINGS_FOLDER, TypeBase, WkbStatus, is_hidden_dir,
};
use crate::error::R;
use crate::info::{read_info, NodeInfo};
use crate::names::{name_parts, is_movable_node, list_subdirs, numeric_id};
use crate::tree_store::{get_node, is_locked_type};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeType {
    pub raw: String,
    pub base: TypeBase,
    pub level: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeActions {
    pub rename: bool,
    pub change_type: bool,
    pub add_next: bool,
    pub move_up: bool,
    pub move_down: bool,
    pub delete: bool,
    pub copy_path: bool,
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
    pub has_children: bool,
    pub is_git: bool,
    pub fixed_prefix: String,
    pub editable_name: String,
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
}

struct Entry {
    name: String,
    on_disk: bool,
    in_yaml: bool,
}

pub fn build_tree(root: &Path, data: &Mapping, yaml_changed: bool) -> R<TreeSnapshot> {
    Ok(TreeSnapshot {
        root: root.to_string_lossy().into_owned(),
        nodes: build_children(root, data, &[])?,
        yaml_changed,
    })
}

pub fn build_children(root: &Path, data: &Mapping, parent: &[String]) -> R<Vec<TreeNode>> {
    let mut nodes = Vec::new();
    for entry in level_entries(root, data, parent)? {
        nodes.push(make_node(root, data, parent, entry)?);
    }
    mark_moves(&mut nodes);
    Ok(nodes)
}

fn level_entries(root: &Path, data: &Mapping, parent: &[String]) -> R<Vec<Entry>> {
    let disk_dir = parent.iter().fold(root.to_path_buf(), |p, part| p.join(part));
    let disk = if disk_dir.is_dir() { list_subdirs(&disk_dir)? } else { Vec::new() };
    let yaml: Vec<&str> = get_node(data, parent)
        .map(|m| child_entries(m).map(|(k, _)| k).collect())
        .unwrap_or_default();
    // A disk folder named like a meta key has no yaml entry of its own, and the trash is never a node.
    let names: BTreeSet<&str> = disk
        .iter()
        .map(String::as_str)
        .filter(|d| !META_KEYS.contains(d))
        .chain(yaml.iter().copied().filter(|k| !is_hidden_dir(k)))
        .collect();

    Ok(names
        .into_iter()
        .map(|name| Entry {
            name: name.to_string(),
            on_disk: disk.iter().any(|d| d == name),
            in_yaml: yaml.contains(&name),
        })
        .filter(|e| !(e.on_disk && !e.in_yaml && is_linked_folder(&e.name)))
        .collect())
}

fn is_linked_folder(name: &str) -> bool {
    LINKED_PREFIXES
        .iter()
        .any(|p| name.strip_prefix(p).is_some_and(|rest| rest.starts_with('.')))
}

// Any of these inside a folder means it holds a git project.
const GIT_MARKERS: [&str; 3] = [".git", ".github", ".gitignore"];

fn make_node(root: &Path, data: &Mapping, parent: &[String], entry: Entry) -> R<TreeNode> {
    let mut rel_parts = parent.to_vec();
    rel_parts.push(entry.name.clone());
    let depth = parent.len();
    let ghost = !entry.on_disk;

    let yaml_entry = get_node(data, &rel_parts);
    let raw_type = yaml_entry.and_then(|m| m.get("type")).and_then(Value::as_str).unwrap_or("");
    let abs_path = rel_parts.iter().fold(root.to_path_buf(), |p, part| p.join(part));
    let info = if ghost { None } else { read_info(&abs_path) };
    // A status in the folder's own info.json is the one shown; the yaml one is the fallback.
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
    let has_children = !ghost && !level_entries(root, data, &rel_parts)?.is_empty();
    let is_git = !ghost && GIT_MARKERS.iter().any(|m| abs_path.join(m).exists());

    let (fixed_prefix, editable_name) = name_parts(&entry.name, is_wkb);
    Ok(TreeNode {
        actions: compute_actions(depth, ghost, raw_type, reserved),
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
        has_children,
        is_git,
        fixed_prefix,
        editable_name,
        info,
        children: None,
        abs_path: abs_path.to_string_lossy().into_owned(),
    })
}

// The only place that decides what a node allows; move flags are filled in by `mark_moves`.
fn compute_actions(depth: usize, ghost: bool, raw_type: &str, reserved: bool) -> NodeActions {
    let locked = is_locked_type(raw_type);
    let frozen = locked || reserved;
    NodeActions {
        rename: !ghost && !frozen,
        change_type: !frozen,
        // Level 1 has its own creation rules, so a new folder is only offered next to a level 2 or deeper one.
        add_next: depth >= 1 && !reserved,
        move_up: false,
        move_down: false,
        delete: !reserved && (ghost || !locked),
        copy_path: !ghost,
    }
}

// Moves swap ids with the nearest movable sibling, so the ends of the movable list cannot go further.
fn mark_moves(nodes: &mut [TreeNode]) {
    let mut movable: Vec<(u32, usize)> = nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| is_movable(n))
        .filter_map(|(i, n)| numeric_id(&n.name).map(|id| (id, i)))
        .collect();
    movable.sort();
    let last = movable.len().saturating_sub(1);
    for (pos, (_, idx)) in movable.iter().enumerate() {
        nodes[*idx].actions.move_up = pos > 0;
        nodes[*idx].actions.move_down = pos < last;
    }
}

fn is_movable(node: &TreeNode) -> bool {
    let raw = node.node_type.as_ref().map_or("", |t| t.raw.as_str());
    !node.ghost && !node.locked && !node.reserved && is_movable_node(&node.name, raw)
}

// Windows folder names ignore case, so "999.settings" is the same reserved folder.
pub(crate) fn is_reserved(rel_parts: &[String]) -> bool {
    matches!(rel_parts, [name]
        if name.eq_ignore_ascii_case(SETTINGS_FOLDER) || name.eq_ignore_ascii_case(REVIEW_FOLDER))
}

fn parse_type(raw: &str) -> NodeType {
    let base = match type_base(raw) {
        "NODE" => TypeBase::Node,
        "WKB" => TypeBase::Wkb,
        "PORTF" => TypeBase::Portf,
        "REV" => TypeBase::Rev,
        "RES" => TypeBase::Res,
        "TPL" => TypeBase::Tpl,
        "CFG" => TypeBase::Cfg,
        _ => TypeBase::Unknown,
    };
    let level = type_level(raw).and_then(|l| l.strip_prefix('L')).and_then(|n| n.parse().ok());
    NodeType { raw: raw.to_string(), base, level }
}

pub(crate) fn parse_status(raw: &str) -> Option<WkbStatus> {
    match raw.trim().to_lowercase().replace(['_', ' '], "-").as_str() {
        "in-progress" => Some(WkbStatus::InProgress),
        "for-review" => Some(WkbStatus::ForReview),
        "hold" => Some(WkbStatus::Hold),
        "done" => Some(WkbStatus::Done),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree_store::test_support::TempDir;

    fn rel(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    fn names(nodes: &[TreeNode]) -> Vec<&str> {
        nodes.iter().map(|n| n.name.as_str()).collect()
    }

    fn fixture() -> (TempDir, Mapping) {
        let dir = TempDir::new("tree");
        for d in [
            "01.Alpha/01.Sub", "01.Alpha/98.Hidden", "02.Beta", "03.Gamma", "04.Wkb",
            "998.Review", "999.Settings/00.Core",
        ] {
            dir.mkdir(d);
        }
        let data: Mapping = serde_yaml::from_str(
            "01.Alpha:\n  type: NODE-L1\n  02.Ghost:\n    type: NODE-L2\n  03.GhostRev:\n    type: REV-L2\n\
             02.Beta:\n  type: NODE-L1\n\
             03.Gamma:\n  type: NODE-L1\n\
             04.Wkb:\n  type: WKB\n  status: In_Progress\n\
             99.Gone:\n  type: NODE-L1\n\
             998.Review:\n  type: REV\n\
             999.Settings:\n  type: NODE-L1\n",
        )
        .unwrap();
        (dir, data)
    }

    fn find<'a>(nodes: &'a [TreeNode], name: &str) -> &'a TreeNode {
        nodes.iter().find(|n| n.name == name).unwrap()
    }

    #[test]
    fn build_tree_merges_disk_and_yaml_sorted_with_ghosts() {
        let (dir, data) = fixture();
        let tree = build_tree(dir.path(), &data, true).unwrap();
        assert!(tree.yaml_changed);
        assert_eq!(
            names(&tree.nodes),
            ["01.Alpha", "02.Beta", "03.Gamma", "04.Wkb", "99.Gone", "998.Review", "999.Settings"]
        );
        let gone = find(&tree.nodes, "99.Gone");
        assert!(gone.ghost && gone.in_yaml && !gone.has_children && gone.children.is_none());
        let alpha = find(&tree.nodes, "01.Alpha");
        assert!(!alpha.ghost && alpha.has_children && alpha.depth == 0);
        let nt = alpha.node_type.as_ref().unwrap();
        assert_eq!((nt.base, nt.level), (TypeBase::Node, Some(1)));
    }

    #[test]
    fn disk_only_linked_dirs_are_hidden_but_other_disk_only_dirs_show() {
        let (dir, data) = fixture();
        let kids = build_children(dir.path(), &data, &rel(&["01.Alpha"])).unwrap();
        assert_eq!(names(&kids), ["01.Sub", "02.Ghost", "03.GhostRev"]);
        let sub = find(&kids, "01.Sub");
        assert!(!sub.in_yaml && !sub.ghost && sub.node_type.is_none());
        assert!(find(&kids, "02.Ghost").ghost);
        assert_eq!(find(&kids, "02.Ghost").depth, 1);

        let data2: Mapping = serde_yaml::from_str("01.Alpha:\n  98.Hidden:\n    type: REV-L2\n").unwrap();
        let kids = build_children(dir.path(), &data2, &rel(&["01.Alpha"])).unwrap();
        assert!(names(&kids).contains(&"98.Hidden"));
    }

    #[test]
    fn status_is_parsed_case_insensitively_for_wkb_only() {
        let (dir, data) = fixture();
        let tree = build_tree(dir.path(), &data, false).unwrap();
        let wkb = find(&tree.nodes, "04.Wkb");
        assert_eq!(wkb.status, Some(WkbStatus::InProgress));
        assert_eq!(wkb.status_raw.as_deref(), Some("In_Progress"));
        assert!(find(&tree.nodes, "02.Beta").status.is_none());
    }

    #[test]
    fn locked_ghost_can_be_deleted_but_not_renamed_or_extended() {
        let (dir, data) = fixture();
        let kids = build_children(dir.path(), &data, &rel(&["01.Alpha"])).unwrap();
        let ghost_rev = find(&kids, "03.GhostRev");
        assert!(ghost_rev.locked && ghost_rev.ghost);
        let a = &ghost_rev.actions;
        assert!(a.delete && !a.rename && !a.change_type && !a.copy_path);
        assert!(!a.move_up && !a.move_down);
    }

    #[test]
    fn ghost_node_can_retype_and_delete_but_not_rename() {
        let (dir, data) = fixture();
        let tree = build_tree(dir.path(), &data, false).unwrap();
        let a = &find(&tree.nodes, "99.Gone").actions;
        assert!(!a.rename && a.change_type && a.delete && !a.add_next && !a.copy_path);
        assert!(!a.move_up && !a.move_down);
    }

    #[test]
    fn locked_real_folder_cannot_be_deleted() {
        let (dir, data) = fixture();
        dir.mkdir("01.Alpha/98.Real");
        let mut data = data;
        let alpha = crate::tree_store::get_node_mut(&mut data, &rel(&["01.Alpha"])).unwrap();
        alpha.insert("98.Real".into(), serde_yaml::from_str("type: REV-L2").unwrap());
        let kids = build_children(dir.path(), &data, &rel(&["01.Alpha"])).unwrap();
        let a = &find(&kids, "98.Real").actions;
        assert!(!a.delete && !a.rename && !a.change_type && a.copy_path);
    }

    #[test]
    fn reserved_root_nodes_allow_nothing_but_copy_path() {
        let (dir, data) = fixture();
        let tree = build_tree(dir.path(), &data, false).unwrap();
        for name in ["999.Settings", "998.Review"] {
            let node = find(&tree.nodes, name);
            assert!(node.reserved, "{name}");
            let a = &node.actions;
            assert!(!a.rename && !a.change_type && !a.add_next, "{name}");
            assert!(!a.move_up && !a.move_down && !a.delete && a.copy_path, "{name}");
        }
        let nested: Mapping = serde_yaml::from_str("01.Alpha:\n  999.Settings:\n    type: NODE-L2\n").unwrap();
        dir.mkdir("01.Alpha/999.Settings");
        let kids = build_children(dir.path(), &nested, &rel(&["01.Alpha"])).unwrap();
        assert!(!find(&kids, "999.Settings").reserved);
    }

    #[test]
    fn reserved_check_ignores_ascii_case() {
        for name in ["999.SETTINGS", "999.settings", "998.REVIEW", "998.review"] {
            assert!(is_reserved(&rel(&[name])), "{name}");
        }
        assert!(!is_reserved(&rel(&["01.A", "998.review"])));
    }

    #[test]
    fn meta_key_folders_and_trash_records_are_hidden() {
        let (dir, mut data) = fixture();
        dir.mkdir("01.Alpha/type");
        dir.mkdir("01.Alpha/status");
        dir.mkdir(".ant-trash/123-02.Old");
        data.insert(".ant-trash".into(), serde_yaml::from_str("type: NODE-L1").unwrap());
        data.insert(".ANT-TRASH".into(), serde_yaml::from_str("type: NODE-L1").unwrap());
        let tree = build_tree(dir.path(), &data, false).unwrap();
        assert!(!names(&tree.nodes).iter().any(|n| n.eq_ignore_ascii_case(".ant-trash")));
        let kids = build_children(dir.path(), &data, &rel(&["01.Alpha"])).unwrap();
        assert_eq!(names(&kids), ["01.Sub", "02.Ghost", "03.GhostRev"]);
    }

    #[test]
    fn movable_nodes_stop_at_the_ends_of_the_movable_list() {
        let (dir, data) = fixture();
        let tree = build_tree(dir.path(), &data, false).unwrap();
        let up_down = |n: &str| {
            let a = &find(&tree.nodes, n).actions;
            (a.move_up, a.move_down)
        };
        assert_eq!(up_down("01.Alpha"), (false, true));
        assert_eq!(up_down("02.Beta"), (true, true));
        assert_eq!(up_down("03.Gamma"), (true, false));
        assert_eq!(up_down("04.Wkb"), (false, false));
    }

    #[test]
    fn a_new_folder_is_only_offered_next_to_level_two_or_deeper() {
        let dir = TempDir::new("depth");
        dir.mkdir("01.A/01.B/01.C");
        let data: Mapping = serde_yaml::from_str(
            "01.A:
  type: NODE-L1
  01.B:
    type: NODE-L2
    01.C:
      type: NODE-L3
",
        )
        .unwrap();
        let top = build_children(dir.path(), &data, &[]).unwrap();
        assert!(!top[0].actions.add_next);
        let l2 = build_children(dir.path(), &data, &rel(&["01.A"])).unwrap();
        assert!(l2[0].actions.add_next);
        let l3 = build_children(dir.path(), &data, &rel(&["01.A", "01.B"])).unwrap();
        assert!(l3[0].actions.add_next);
    }

    #[test]
    fn snapshot_serialises_in_camel_case() {
        let (dir, data) = fixture();
        let tree = build_tree(dir.path(), &data, false).unwrap();
        let text = serde_yaml::to_string(&tree).unwrap();
        for key in ["yamlChanged", "relParts", "inYaml", "hasChildren", "absPath", "changeType", "copyPath"] {
            assert!(text.contains(key), "{key}");
        }
    }
}
