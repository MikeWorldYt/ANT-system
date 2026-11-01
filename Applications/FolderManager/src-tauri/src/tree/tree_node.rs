// Tree rows sent to the UI: disk folders merged with tree entries, plus the per-node action rules.
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use serde::Serialize;
use serde_yaml::{Mapping, Value};

use crate::ant_core::tree_types::{child_entries, flags_in, type_base, type_level, META_KEYS};
use crate::config::{
    HIDDEN_FLAG, LINK_FAVORITES_FILENAME, LINK_HIDDEN_FILENAME, LINKED_PREFIXES, MAIN_FLAG, REVIEW_FOLDER, SETTINGS_FOLDER, TypeBase, WkbStatus, is_hidden_dir,
};
use crate::error::R;
use crate::fs_safe::{io_err, join_rel};
use crate::index::footprint::Copied;
use crate::index::{link_list, review_index};
use crate::storage::info::{field_of, read_info, NodeInfo};
use crate::ant_core::names::{name_parts, is_movable_node, is_system_root_name, list_subdirs, numeric_id};
use crate::tree::depth;
use crate::tree::tree_store::{get_node, get_type, is_locked_type};

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
    pub add_child: bool,
    pub move_up: bool,
    pub move_down: bool,
    pub delete: bool,
    pub copy_path: bool,
    pub open: bool,
    pub favorite: bool,
    pub edit_details: bool,
    pub edit_flags: bool,
    pub hide_link: bool,
    pub hide: bool,
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
}

struct Entry {
    name: String,
    on_disk: bool,
    in_yaml: bool,
    link: Option<Vec<String>>,
    favorite: bool,
    hidden: bool,
    file: bool,
}

pub fn build_tree(root: &Path, data: &Mapping, yaml_changed: bool) -> R<TreeSnapshot> {
    Ok(TreeSnapshot {
        root: root.to_string_lossy().into_owned(),
        nodes: children_at(root, data, &[], depth::limit(root)?)?,
        yaml_changed,
        copies: Vec::new(),
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
        if node.has_children && node.depth < limit {
            let mut kids = children_at(root, data, &node.rel_parts, limit)?;
            fill_at(root, data, &mut kids, limit)?;
            node.children = Some(kids);
        }
    }
    Ok(())
}

// A folder on disk with no database entry, and not a review link.
fn is_unmanaged(entry: &Entry) -> bool {
    entry.on_disk && !entry.in_yaml && entry.link.is_none() && !entry.file
}

// Below a review folder past the limit the database has no entries, so its unmanaged folders are only browsed.
fn under_review_past_limit(data: &Mapping, rel: &[String], limit: usize) -> bool {
    (limit..rel.len()).any(|k| type_base(&get_type(data, &rel[..k])) == "REV")
}

fn level_entries(root: &Path, data: &Mapping, parent: &[String]) -> R<Vec<Entry>> {
    let disk_dir = join_rel(root, parent);
    let disk = if disk_dir.is_dir() { list_subdirs(&disk_dir)? } else { Vec::new() };
    let yaml: Vec<&str> = get_node(data, parent)
        .map(|m| child_entries(m).map(|(k, _)| k).collect())
        .unwrap_or_default();
    // A hidden folder keeps its yaml entry but is not listed, so nothing below it is read either.
    let hidden_here: BTreeSet<&str> = get_node(data, parent)
        .map(|m| child_entries(m).filter(|(_, c)| flags_in(c).iter().any(|f| f == HIDDEN_FLAG)).map(|(k, _)| k).collect())
        .unwrap_or_default();
    // A disk folder named like a meta key has no yaml entry of its own, and the trash is never a node.
    let names: BTreeSet<&str> = disk
        .iter()
        .map(String::as_str)
        .filter(|d| !META_KEYS.contains(d))
        .chain(yaml.iter().copied().filter(|k| !is_hidden_dir(k)))
        .filter(|name| !hidden_here.contains(name))
        .filter(|name| !(parent.is_empty() && is_hidden_system_root(name)))
        .collect();
    let link_entries: Vec<Entry> = if matches!(parent, [top] if top.eq_ignore_ascii_case(REVIEW_FOLDER)) {
        let starred = link_list::read(root, LINK_FAVORITES_FILENAME)?;
        let hidden_links = link_list::read(root, LINK_HIDDEN_FILENAME)?;
        let hidden = review_index::hidden_categories(root);
        let favorites_shown = review_index::favorites_shown(root);
        let links = review_index::links(root, data)?;
        // A starred link brings its category's top link along, so the group keeps its heading under a hidden category.
        let starred_categories: Vec<String> = links
            .iter()
            .filter(|(name, _)| favorites_shown && starred.contains(name))
            .map(|(_, owner)| owner[0].clone())
            .collect();
        links
            .into_iter()
            .filter(|(name, owner)| {
                let kept_as_favorite = favorites_shown && starred.contains(name);
                let kept_as_heading = owner.len() == 1 && starred_categories.contains(&owner[0]);
                (kept_as_favorite || kept_as_heading || !hidden.contains(&owner[0])) && !names.contains(name.as_str())
            })
            .map(|(name, owner)| Entry { favorite: starred.contains(&name), hidden: hidden_links.contains(&name), name, on_disk: true, in_yaml: false, link: Some(owner), file: false })
            .collect()
    } else {
        Vec::new()
    };

    let file_entries: Vec<Entry> = if lists_files(parent) {
        disk_files(&disk_dir)?
            .into_iter()
            .map(|name| Entry { name, on_disk: true, in_yaml: false, link: None, favorite: false, hidden: false, file: true })
            .collect()
    } else {
        Vec::new()
    };
    let mut entries: Vec<Entry> = names
        .into_iter()
        .map(|name| Entry {
            name: name.to_string(),
            on_disk: disk.iter().any(|d| d == name),
            in_yaml: yaml.contains(&name),
            link: None,
            favorite: false,
            hidden: false,
            file: false,
        })
        .filter(|e| !(e.on_disk && !e.in_yaml && is_linked_folder(&e.name)))
        .chain(link_entries)
        .chain(file_entries)
        .collect();
    entries.sort_by_cached_key(|e| (is_tail(&e.name), natural_key(&e.name)));
    Ok(entries)
}

// Compares the first run of digits as a number, so PJ.999 comes before PJ.1000.
fn natural_key(name: &str) -> (String, u64, String) {
    let start = name.find(|c: char| c.is_ascii_digit()).unwrap_or(name.len());
    let digits = name[start..].bytes().take_while(u8::is_ascii_digit).count();
    let number = name[start..start + digits].parse().unwrap_or(0);
    (name[..start].to_string(), number, name[start + digits..].to_string())
}

// The 90-99 folders and the 998/999 ones always close a list, after workboards and unnumbered folders too.
fn is_tail(name: &str) -> bool {
    numeric_id(name).is_some_and(|n| (90..=99).contains(&n) || n == 998 || n == 999)
}

fn is_linked_folder(name: &str) -> bool {
    LINKED_PREFIXES
        .iter()
        .any(|p| name.strip_prefix(p).is_some_and(|rest| rest.starts_with('.')))
}

// Any of these inside a folder means it holds a git project.
const GIT_MARKERS: [&str; 3] = [".git", ".github", ".gitignore"];

// Review folders and workboards hold files, so they open even when empty.
pub(crate) fn holds_files(data: &Mapping, rel: &[String]) -> bool {
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
        hidden: false,
        file: true,
        info: None,
        children: None,
        actions: NodeActions {
            rename: true,
            change_type: false,
            add_next: false,
            add_child: false,
            move_up: false,
            move_down: false,
            delete: true,
            copy_path: true,
            open: true,
            favorite: false,
            edit_details: false,
            edit_flags: false,
            hide_link: false,
            hide: false,
        },
        name,
    }
}

// Review folders also list their files; hidden files and the Windows desktop.ini stay out.
fn disk_files(dir: &Path) -> R<Vec<String>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| io_err("read the folder", dir, &e))? {
        let entry = entry.map_err(|e| io_err("read the folder", dir, &e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name.eq_ignore_ascii_case("desktop.ini") || is_running_app(dir, &entry.file_name()) {
            continue;
        }
        if entry.file_type().map_err(|e| io_err("read the folder", dir, &e))?.is_file() {
            files.push(name);
        }
    }
    Ok(files)
}

// The app can sit inside the storage it manages; listing its own executable would show the app inside itself.
fn is_running_app(dir: &Path, name: &OsStr) -> bool {
    let Ok(exe) = std::env::current_exe() else { return false };
    exe.file_name().is_some_and(|exe_name| exe_name.eq_ignore_ascii_case(name))
        && exe.parent().is_some_and(|parent| fs::canonicalize(parent).ok() == fs::canonicalize(dir).ok())
}

fn make_node(root: &Path, data: &Mapping, parent: &[String], entry: Entry, limit: usize) -> R<TreeNode> {
    let mut rel_parts = parent.to_vec();
    rel_parts.push(entry.name.clone());
    let depth = parent.len();
    let ghost = !entry.on_disk;
    let linked = entry.link.is_some();

    let yaml_entry = get_node(data, &rel_parts);
    let raw_type = yaml_entry.and_then(|m| m.get("type")).and_then(Value::as_str).unwrap_or("");
    // A review link stands for its owner's review folder, so that is where its path and drops point.
    let abs_path = match &entry.link {
        Some(owner) => review_index::rev_folder(root, owner),
        None => join_rel(root, &rel_parts),
    };
    let info = if ghost || linked { None } else { read_info(&abs_path) };
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
    // Folders on the last tree level show no children, so the views offer nothing to open below them.
    // The review root stays a container even when its filter hides every link, so its empty list can say why.
    let review_root = depth == 0 && entry.name.eq_ignore_ascii_case(REVIEW_FOLDER);
    // A review folder always opens, even when empty, so its files and the Empty legend can show.
    let content_folder = !ghost && !linked && matches!(type_base(raw_type), "REV" | "WKB" | "OUT");
    let browsable = is_unmanaged(&entry) && under_review_past_limit(data, &rel_parts, limit);
    let has_children = !ghost && (review_root || content_folder || (!linked && (depth + 1 < limit || browsable) && !level_entries(root, data, &rel_parts)?.is_empty()));
    let is_git = !ghost && !linked && GIT_MARKERS.iter().any(|m| abs_path.join(m).exists());

    let info_broken = info.as_ref().is_some_and(|i| i.error.is_some());

    let (fixed_prefix, editable_name) = name_parts(&entry.name, is_wkb);
    Ok(TreeNode {
        actions: if linked { link_actions() } else { compute_actions(depth, limit, ghost, raw_type, reserved, info_broken) },
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
        link_path: entry.link.as_deref().map(review_index::rev_path),
        flags: node_flags(depth, yaml_entry),
        has_children,
        is_git,
        fixed_prefix,
        editable_name,
        favorite: entry.favorite || info.as_ref().and_then(|i| field_of(i, &["favorite"])).is_some_and(|v| v.eq_ignore_ascii_case("true")),
        hidden: entry.hidden,
        file: false,
        info,
        children: None,
        abs_path: abs_path.to_string_lossy().into_owned(),
    })
}

// A review link only opens its owner folder; files reach the owner through the drop target instead.
fn link_actions() -> NodeActions {
    NodeActions {
        rename: false,
        change_type: false,
        add_next: false,
        add_child: false,
        move_up: false,
        move_down: false,
        delete: false,
        copy_path: true,
        open: true,
        favorite: true,
        edit_details: false,
        edit_flags: false,
        hide_link: true,
        hide: false,
    }
}

// The only place that decides what a node allows; move flags are filled in by `mark_moves`.
fn compute_actions(depth: usize, limit: usize, ghost: bool, raw_type: &str, reserved: bool, info_broken: bool) -> NodeActions {
    let locked = is_locked_type(raw_type);
    let frozen = locked || reserved;
    // The yaml stops at the depth limit, so deeper folders have no entry to retype or to add a sibling under.
    let in_tree = depth < limit;
    let outcome = type_base(raw_type) == "OUT";
    NodeActions {
        rename: !ghost && !frozen,
        change_type: !frozen && in_tree && type_base(raw_type) != "OUT",
        // Main categories can create another Level 1 category; deeper nodes create siblings as usual.
        add_next: !ghost && !reserved && in_tree,
        // Mirrors add_node: the new child sits one level below this node and must stay inside the limit.
        add_child: !ghost && !frozen && depth + 1 < limit,
        move_up: false,
        move_down: false,
        delete: !reserved && (ghost || !locked) && !outcome,
        copy_path: !ghost,
        open: !ghost,
        // A broken info.json is never overwritten, so neither edit can be saved into it.
        favorite: !reserved && !ghost && !info_broken && !outcome,
        edit_details: !frozen && !ghost && !info_broken && type_base(raw_type) == "WKB",
        edit_flags: !frozen && !ghost && depth > 0 && type_base(raw_type) == "NODE",
        hide_link: false,
        hide: !frozen && !ghost && depth > 0,
    }
}

// Root folders are always main, a rule rather than a stored flag; the rest carry what the user set.
fn node_flags(depth: usize, yaml_entry: Option<&Mapping>) -> Vec<String> {
    let mut flags = if depth == 0 { vec![MAIN_FLAG.to_string()] } else { Vec::new() };
    flags.extend(yaml_entry.map(flags_in).unwrap_or_default());
    flags
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
    !node.file && !node.ghost && !node.locked && !node.reserved && !node.linked && is_movable_node(&node.name, raw)
}

// Windows folder names ignore case, so "999.settings" is the same reserved folder; the whole 9xx range is the app's.
pub(crate) fn is_reserved(rel_parts: &[String]) -> bool {
    matches!(rel_parts, [name]
        if name.eq_ignore_ascii_case(SETTINGS_FOLDER) || name.eq_ignore_ascii_case(REVIEW_FOLDER) || is_system_root_name(name))
}

// The 9xx root folders are the app's; 998.Review stays listed because its links are what the user browses.
fn is_hidden_system_root(name: &str) -> bool {
    is_system_root_name(name) && !name.eq_ignore_ascii_case(REVIEW_FOLDER)
}

fn parse_type(raw: &str) -> NodeType {
    let base = TypeBase::from_str(type_base(raw)).unwrap_or(TypeBase::Unknown);
    let level = type_level(raw).and_then(|l| l.strip_prefix('L')).and_then(|n| n.parse().ok());
    NodeType { raw: raw.to_string(), base, level }
}

pub(crate) fn parse_status(raw: &str) -> Option<WkbStatus> {
    WkbStatus::from_str(&raw.trim().to_lowercase().replace(['_', ' '], "-"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{storage, TempDir};

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
    fn workboard_numbers_sort_as_numbers() {
        let mut names = vec!["PJ.1000_B", "PJ.999_A", "PJ.021_C", "PJ.001_D"];
        names.sort_by_cached_key(|n| natural_key(n));
        assert_eq!(names, ["PJ.001_D", "PJ.021_C", "PJ.999_A", "PJ.1000_B"]);
    }

    #[test]
    fn build_tree_merges_disk_and_yaml_sorted_with_ghosts() {
        let (dir, data) = fixture();
        let tree = build_tree(dir.path(), &data, true).unwrap();
        assert!(tree.yaml_changed);
        assert_eq!(
            names(&tree.nodes),
            ["01.Alpha", "02.Beta", "03.Gamma", "04.Wkb", "99.Gone", "998.Review"]
        );
        let gone = find(&tree.nodes, "99.Gone");
        assert!(gone.ghost && gone.in_yaml && !gone.has_children && gone.children.is_none());
        let alpha = find(&tree.nodes, "01.Alpha");
        assert!(!alpha.ghost && alpha.has_children && alpha.depth == 0);
        let nt = alpha.node_type.as_ref().unwrap();
        assert_eq!((nt.base, nt.level), (TypeBase::Node, Some(1)));
    }

    #[test]
    fn the_90s_and_the_last_roots_close_the_list_after_workboards() {
        let dir = TempDir::new("tail");
        for d in ["01.A", "98.Review", "90.Resources", "PJ.001_Wb", "Loose"] {
            dir.mkdir(&format!("01.Top/{d}"));
        }
        let data: Mapping = serde_yaml::from_str(
            "01.Top:
  type: NODE-L1
  01.A: {type: NODE-L2}
  98.Review: {type: REV-L1}
  90.Resources: {type: RES}
  PJ.001_Wb: {type: WKB-L2}
",
        )
        .unwrap();
        let kids = build_children(dir.path(), &data, &rel(&["01.Top"])).unwrap();
        assert_eq!(names(&kids), ["01.A", "Loose", "PJ.001_Wb", "90.Resources", "98.Review"]);
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
        assert!(a.delete && !a.rename && !a.change_type && !a.copy_path && !a.add_child);
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
        let alpha = crate::tree::tree_store::get_node_mut(&mut data, &rel(&["01.Alpha"])).unwrap();
        alpha.insert("98.Real".into(), serde_yaml::from_str("type: REV-L2").unwrap());
        let kids = build_children(dir.path(), &data, &rel(&["01.Alpha"])).unwrap();
        let a = &find(&kids, "98.Real").actions;
        assert!(!a.delete && !a.rename && !a.change_type && !a.add_child && a.copy_path);
    }

    #[test]
    fn system_root_folders_stay_out_of_the_tree_but_nested_ones_show() {
        let (dir, data) = fixture();
        let tree = build_tree(dir.path(), &data, false).unwrap();
        assert!(!names(&tree.nodes).contains(&"999.Settings"));
        assert!(find(&tree.nodes, "998.Review").reserved);
        let nested: Mapping = serde_yaml::from_str("01.Alpha:\n  999.Settings:\n    type: NODE-L2\n").unwrap();
        dir.mkdir("01.Alpha/999.Settings");
        let kids = build_children(dir.path(), &nested, &rel(&["01.Alpha"])).unwrap();
        assert!(!find(&kids, "999.Settings").reserved);
    }

    #[test]
    fn review_links_are_leaf_folders_that_open_their_owner() {
        let (dir, data) = fixture();
        dir.mkdir("02.Beta/98.Beta");
        std::fs::write(dir.path().join("998.Review/02.Beta.lnk"), "").unwrap();
        let kids = build_children(dir.path(), &data, &rel(&["998.Review"])).unwrap();
        let link = find(&kids, "02.Beta");
        assert!(link.linked && !link.ghost && !link.has_children && link.node_type.is_none());
        assert_eq!(std::path::PathBuf::from(&link.abs_path), dir.path().join("02.Beta/98.Beta"));
        let a = &link.actions;
        assert!(a.open && a.copy_path && a.favorite && !a.rename && !a.delete && !a.move_up && !a.move_down);
        assert!(!link.favorite);
        dir.mkdir(".ant");
        std::fs::write(dir.path().join(".ant/lnk-favorites.json"), "[\"02.Beta\"]\n").unwrap();
        let kids = build_children(dir.path(), &data, &rel(&["998.Review"])).unwrap();
        assert!(find(&kids, "02.Beta").favorite);
    }

    #[test]
    fn an_empty_outcome_still_opens_and_cannot_be_favorited_or_deleted() {
        let dir = TempDir::new("tree-outcome");
        dir.mkdir("100.WORK/02.Beta/00.Outcomes");
        let mut data = Mapping::new();
        crate::tree::tree_store::sync(dir.path(), &mut data).unwrap();
        let kids = build_children(dir.path(), &data, &rel(&["100.WORK", "02.Beta"])).unwrap();
        let out = find(&kids, "00.Outcomes");
        assert!(out.has_children);
        assert!(!out.actions.favorite && !out.actions.delete && out.actions.open);
    }

    #[test]
    fn the_review_root_stays_a_container_when_no_link_is_listed() {
        let (dir, data) = fixture();
        let tree = build_tree(dir.path(), &data, false).unwrap();
        assert!(find(&tree.nodes, "998.Review").has_children);
    }

    #[test]
    fn reserved_check_ignores_ascii_case() {
        for name in ["999.SETTINGS", "999.settings", "998.REVIEW", "998.review"] {
            assert!(is_reserved(&rel(&[name])), "{name}");
        }
        assert!(!is_reserved(&rel(&["01.A", "998.review"])));
    }

    #[test]
    fn review_folders_always_open_and_list_their_files() {
        let dir = storage("review-files", &["100.A/98.A", "100.A/98.B"], "100.A:\n  98.A:\n    type: REV-L2\n  98.B:\n    type: REV-L2\n");
        std::fs::write(dir.path().join("100.A/98.A/notes.txt"), "x").unwrap();
        std::fs::write(dir.path().join("100.A/98.A/.hidden"), "x").unwrap();
        std::fs::write(dir.path().join("100.A/98.A/desktop.ini"), "x").unwrap();
        let data = crate::test_support::yaml(dir.path());
        let folders = build_children(dir.path(), &data, &rel(&["100.A"])).unwrap();
        let review = folders.iter().find(|n| n.name == "98.A").unwrap();
        assert!(review.has_children);
        let files = build_children(dir.path(), &data, &review.rel_parts).unwrap();
        assert_eq!(names(&files), ["notes.txt"]);
        assert!(files[0].file && !files[0].has_children && files[0].actions.rename && !files[0].actions.favorite);
        let empty = folders.iter().find(|n| n.name == "98.B").unwrap();
        assert!(empty.has_children);
        assert!(build_children(dir.path(), &data, &empty.rel_parts).unwrap().is_empty());
    }

    #[test]
    fn folders_past_a_review_limit_show_for_browsing_without_database_actions() {
        let dir = storage("review-browse", &["100.A/01.B/98.B/New folder/Inside"], "100.A:\n  01.B:\n    type: NODE-L2\n    98.B: {type: REV-L3}\n");
        std::fs::write(dir.path().join("100.A/01.B/98.B/notes.txt"), "x").unwrap();
        let data = crate::test_support::yaml(dir.path());
        let kids = build_children(dir.path(), &data, &rel(&["100.A", "01.B", "98.B"])).unwrap();
        assert_eq!(names(&kids), ["New folder", "notes.txt"]);
        let folder = find(&kids, "New folder");
        assert!(folder.has_children && folder.node_type.is_none() && !folder.file);
        assert_eq!(names(&build_children(dir.path(), &data, &folder.rel_parts).unwrap()), ["Inside"]);
    }

    #[test]
    fn every_folder_lists_its_files_but_reserved_folders_do_not() {
        let dir = storage("files-anywhere", &["100.A/02.B"], "100.A:\n  type: NODE-L1\n  02.B: {type: NODE-L2}\n");
        std::fs::write(dir.path().join("100.A/02.B/photo.png"), "x").unwrap();
        let data = crate::test_support::yaml(dir.path());
        let kids = build_children(dir.path(), &data, &rel(&["100.A"])).unwrap();
        let node = kids.iter().find(|n| n.name == "02.B").unwrap();
        assert!(node.has_children);
        assert_eq!(names(&build_children(dir.path(), &data, &node.rel_parts).unwrap()), ["photo.png"]);
        assert!(build_children(dir.path(), &data, &rel(&["998.Review"])).unwrap().iter().all(|n| !n.file));
    }

    #[test]
    fn workboards_open_with_their_files_and_hold_no_review_folder() {
        let dir = storage("wkb-files", &["100.A/PJ.001_X"], "100.A:\n  PJ.001_X: {type: WKB-L2}\n");
        std::fs::write(dir.path().join("100.A/PJ.001_X/brief.txt"), "x").unwrap();
        let data = crate::test_support::yaml(dir.path());
        let kids = build_children(dir.path(), &data, &rel(&["100.A"])).unwrap();
        let wkb = kids.iter().find(|n| n.name == "PJ.001_X").unwrap();
        assert!(wkb.has_children);
        assert_eq!(names(&build_children(dir.path(), &data, &wkb.rel_parts).unwrap()), ["brief.txt"]);
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
        assert!(top[0].actions.add_next && top[0].actions.add_child);
        let l2 = build_children(dir.path(), &data, &rel(&["01.A"])).unwrap();
        assert!(l2[0].actions.add_next && l2[0].actions.add_child);
        let l3 = build_children(dir.path(), &data, &rel(&["01.A", "01.B"])).unwrap();
        assert!(l3[0].actions.add_next && !l3[0].actions.add_child);
    }

    #[test]
    fn folders_below_the_yaml_depth_cannot_be_retyped_or_extended() {
        let dir = TempDir::new("too-deep");
        dir.mkdir("01.A/01.B/01.C/01.D/01.E");
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
        let l3 = build_children(dir.path(), &data, &rel(&["01.A", "01.B"])).unwrap();
        assert!(l3[0].actions.change_type && l3[0].actions.add_next);
        let l4 = build_children(dir.path(), &data, &rel(&["01.A", "01.B", "01.C"])).unwrap();
        let a = &l4[0].actions;
        assert_eq!(l4[0].depth, crate::config::MAX_DEPTH);
        assert!(!a.change_type && !a.add_next && a.rename && a.copy_path);
        assert!(!l4[0].has_children);
    }

    #[test]
    fn a_raised_depth_limit_loads_and_opens_the_next_level() {
        let dir = TempDir::new("raised-depth");
        dir.mkdir("01.A/01.B/01.C/01.D/01.E");
        crate::storage::db::Store::open_or_create(dir.path()).unwrap().set_setting(depth::DEPTH_KEY, "4").unwrap();
        let tree = crate::tree::snapshot(dir.path(), crate::index::footprint::Copies::Tail).unwrap();
        let child = |n: &TreeNode| n.children.as_ref().map(|k| k[0].clone()).unwrap();
        let l3 = child(&child(&tree.nodes[0]));
        assert!(l3.has_children && l3.actions.change_type && l3.actions.add_next);
        let l4 = child(&l3);
        assert_eq!((l4.name.as_str(), l4.depth), ("01.D", 3));
        assert!(!l4.has_children && l4.actions.change_type && l4.actions.add_next);
        let data = crate::tree::tree_store::load(dir.path()).unwrap();
        assert_eq!(crate::tree::tree_store::get_type(&data, &rel(&["01.A", "01.B", "01.C", "01.D"])), "NODE-L4");
        assert!(get_node(&data, &rel(&["01.A", "01.B", "01.C", "01.D", "01.E"])).is_none());
    }

    #[test]
    fn fill_children_stops_at_the_last_tree_level() {
        let (dir, data) = fixture();
        dir.mkdir("01.Alpha/01.Sub/01.Deep/01.Deeper");
        let mut tree = build_tree(dir.path(), &data, false).unwrap();
        fill_children(dir.path(), &data, &mut tree.nodes).unwrap();
        let alpha = find(&tree.nodes, "01.Alpha");
        let sub = find(alpha.children.as_ref().unwrap(), "01.Sub");
        let deep = find(sub.children.as_ref().unwrap(), "01.Deep");
        assert!(!deep.has_children && deep.children.is_none());
        assert!(find(&tree.nodes, "02.Beta").children.is_none());
    }

    #[test]
    fn favorite_and_details_follow_what_their_commands_accept() {
        let (dir, data) = fixture();
        dir.mkdir("03.Gamma/.ant");
        std::fs::write(dir.path().join("03.Gamma/.ant/info.json"), "{oops").unwrap();
        let tree = build_tree(dir.path(), &data, false).unwrap();
        let acts = |n: &str| {
            let a = &find(&tree.nodes, n).actions;
            (a.favorite, a.edit_details)
        };
        assert_eq!(acts("04.Wkb"), (true, true));
        assert_eq!(acts("02.Beta"), (true, false));
        assert_eq!(acts("03.Gamma"), (false, false));
        assert_eq!(acts("99.Gone"), (false, false));
        dir.mkdir("04.Wkb/.ant");
        std::fs::write(dir.path().join("04.Wkb/.ant/info.json"), "[1]").unwrap();
        let tree = build_tree(dir.path(), &data, false).unwrap();
        let a = &find(&tree.nodes, "04.Wkb").actions;
        assert!(!a.favorite && !a.edit_details);
        assert!(!find(&tree.nodes, "02.Beta").actions.edit_flags && !find(&tree.nodes, "04.Wkb").actions.edit_flags);
    }

    #[test]
    fn snapshot_serialises_in_camel_case() {
        let (dir, data) = fixture();
        let tree = build_tree(dir.path(), &data, false).unwrap();
        let text = serde_yaml::to_string(&tree).unwrap();
        for key in ["yamlChanged", "relParts", "inYaml", "hasChildren", "absPath", "changeType", "copyPath", "editDetails", "addChild"] {
            assert!(text.contains(key), "{key}");
        }
    }
}
