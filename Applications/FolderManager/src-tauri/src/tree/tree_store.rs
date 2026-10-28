// Tree access on ant.db: load, save, sync with the folders on disk and edit entries by path.
use std::path::Path;

use serde_yaml::{Mapping, Value};

use crate::ant_core::tree_types::{type_base, META_KEYS};
use crate::config::{DATA_FLAG, LOCKED_TYPES, MAX_DEPTH, SPECIAL_TYPES, WKB_IDENTIFIERS};
use crate::storage::db::{db_path, Store};
use crate::error::R;
use crate::fs_safe::join_rel;
use crate::ant_core::names::list_subdirs;

pub fn load(root: &Path) -> R<Mapping> {
    match Store::open_existing(root)? {
        Some(store) => store.read(),
        None => Ok(Mapping::new()),
    }
}

pub fn save(root: &Path, data: &Mapping) -> R<()> {
    Store::open_or_create(root)?.replace(&sort_mapping(data))
}

pub fn infer_type(name: &str, depth: usize) -> String {
    let basename = name.rsplit(['/', '\\']).next().unwrap_or(name);
    if WKB_IDENTIFIERS.iter().any(|p| basename.starts_with(p)) {
        return "WKB".to_string();
    }
    match SPECIAL_TYPES.iter().find(|(prefix, _)| basename.starts_with(prefix)) {
        Some((_, folder_type)) => (*folder_type).to_string(),
        None => format!("NODE-L{}", depth + 1),
    }
}

pub fn is_locked_type(t: &str) -> bool {
    LOCKED_TYPES.contains(&type_base(t))
}

// Disk dirs missing from the tree are added; tree-only keys stay as ghosts.
// A storage with no db yet is new, so its Node folders on level 3 start with the data flag.
pub fn sync(root: &Path, data: &mut Mapping) -> R<bool> {
    let fresh = !db_path(root).is_file();
    sync_level(root, data, 0, fresh)
}

pub fn get_node<'a>(data: &'a Mapping, rel: &[String]) -> Option<&'a Mapping> {
    let mut node = data;
    for part in rel {
        node = node.get(part.as_str())?.as_mapping()?;
    }
    Some(node)
}

pub fn get_node_mut<'a>(data: &'a mut Mapping, rel: &[String]) -> Option<&'a mut Mapping> {
    let mut node = data;
    for part in rel {
        node = node.get_mut(part.as_str())?.as_mapping_mut()?;
    }
    Some(node)
}

pub fn get_type(data: &Mapping, rel: &[String]) -> String {
    get_node(data, rel)
        .and_then(|m| m.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

// Only child entries (mappings) move or go, so a node's own type or status can never be touched.
pub fn rename_key(data: &mut Mapping, parent: &[String], old: &str, new: &str) -> bool {
    let Some(node) = get_node_mut(data, parent) else { return false };
    if META_KEYS.contains(&new) || !matches!(node.get(old), Some(Value::Mapping(_))) {
        return false;
    }
    match node.remove(old) {
        Some(entry) => {
            node.insert(Value::String(new.to_string()), entry);
            true
        }
        None => false,
    }
}

pub fn delete_key(data: &mut Mapping, parent: &[String], key: &str) -> bool {
    let Some(node) = get_node_mut(data, parent) else { return false };
    matches!(node.get(key), Some(Value::Mapping(_))) && node.remove(key).is_some()
}

// Syncs only the levels from the root down to `dir`, one listing each, without descending any further.
pub fn sync_branch(root: &Path, data: &mut Mapping, dir: &[String]) -> R<bool> {
    let fresh = !db_path(root).is_file();
    let (mut node, mut path, mut changed) = (data, root.to_path_buf(), false);
    for depth in 0..=dir.len().min(MAX_DEPTH - 1) {
        let names = list_subdirs(&path)?;
        for name in names.iter().filter(|n| !META_KEYS.contains(&n.as_str())) {
            changed |= add_missing(node, name, depth, fresh);
        }
        // Exact names only, as a full sync only descends into folders listed on disk.
        let Some(part) = dir.get(depth).filter(|p| names.contains(p)) else { break };
        let Some(Value::Mapping(child)) = node.get_mut(part.as_str()) else { break };
        node = child;
        path.push(part);
    }
    Ok(changed)
}

// Syncs a node's whole subtree; a missing folder has nothing to add.
pub fn sync_subtree(root: &Path, rel: &[String], node: &mut Mapping) -> R<bool> {
    let dir = join_rel(root, rel);
    if !dir.is_dir() {
        return Ok(false);
    }
    sync_level(&dir, node, rel.len(), !db_path(root).is_file())
}

fn sync_level(dir: &Path, node: &mut Mapping, depth: usize, fresh: bool) -> R<bool> {
    if depth >= MAX_DEPTH {
        return Ok(false);
    }
    let mut changed = false;
    for name in list_subdirs(dir)? {
        // A folder named like a meta key would overwrite the node's own type or status.
        if META_KEYS.contains(&name.as_str()) {
            continue;
        }
        changed |= add_missing(node, &name, depth, fresh);
        if let Some(Value::Mapping(child)) = node.get_mut(name.as_str()) {
            changed |= sync_level(&dir.join(&name), child, depth + 1, fresh)?;
        }
    }
    Ok(changed)
}

fn add_missing(node: &mut Mapping, name: &str, depth: usize, fresh: bool) -> bool {
    if matches!(node.get(name), Some(Value::Mapping(_))) {
        return false;
    }
    let kind = infer_type(name, depth);
    let mut entry = Mapping::new();
    if fresh && kind == "NODE-L3" {
        entry.insert("flags".into(), DATA_FLAG.into());
    }
    entry.insert("type".into(), kind.into());
    node.insert(Value::String(name.to_string()), Value::Mapping(entry));
    true
}

// Siblings are stored in byte order of their names so the tree reads back the same way every time.
fn sort_mapping(m: &Mapping) -> Mapping {
    let mut entries: Vec<(&Value, &Value)> = m.iter().collect();
    entries.sort_by(|a, b| a.0.as_str().cmp(&b.0.as_str()));
    entries.into_iter().map(|(k, v)| (k.clone(), sort_value(v))).collect()
}

fn sort_value(v: &Value) -> Value {
    match v {
        Value::Mapping(m) => Value::Mapping(sort_mapping(m)),
        Value::Sequence(items) => Value::Sequence(items.iter().map(sort_value).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::TempDir;
    use super::*;

    fn parse(s: &str) -> Mapping {
        serde_yaml::from_str(s).unwrap()
    }

    fn rel(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn load_without_a_db_gives_empty_mapping() {
        let dir = TempDir::new("load-missing");
        assert!(load(dir.path()).unwrap().is_empty());
        assert!(!dir.path().join(".ant/ant.db").exists());
    }

    #[test]
    fn save_then_load_round_trips_with_sorted_keys() {
        let dir = TempDir::new("roundtrip");
        let data = parse("zeta:\n  type: NODE-L1\n  b: {type: X}\n  a: {type: Y}\nalpha:\n  type: REV\n");
        save(dir.path(), &data).unwrap();
        let loaded = load(dir.path()).unwrap();
        let order: Vec<_> = loaded.keys().filter_map(Value::as_str).collect();
        assert_eq!(order, ["alpha", "zeta"]);
        let children: Vec<_> = get_node(&loaded, &rel(&["zeta"])).unwrap().keys().filter_map(Value::as_str).collect();
        assert_eq!(children, ["type", "a", "b"]);
        assert_eq!(get_type(&loaded, &rel(&["zeta", "a"])), "Y");
        assert_eq!(get_type(&loaded, &rel(&["alpha"])), "REV");
        assert!(!dir.path().join("999.Settings").exists());
    }

    #[test]
    fn sync_adds_new_dirs_keeps_ghosts_and_settles() {
        let dir = TempDir::new("sync");
        dir.mkdir("01.Math/PJ.Demo");
        dir.mkdir("01.Math/PJ.Demo/deep/deeper");
        dir.mkdir("90.Resources");
        dir.mkdir(".ant-trash/x");
        let mut data = parse("ZZ.Ghost:\n  type: NODE-L1\n01.Math: null\n");
        assert!(sync(dir.path(), &mut data).unwrap());
        assert_eq!(get_type(&data, &rel(&["ZZ.Ghost"])), "NODE-L1");
        assert_eq!(get_type(&data, &rel(&["01.Math"])), "NODE-L1");
        assert_eq!(get_type(&data, &rel(&["01.Math", "PJ.Demo"])), "WKB");
        assert_eq!(get_type(&data, &rel(&["90.Resources"])), "RES");
        assert_eq!(get_type(&data, &rel(&["01.Math", "PJ.Demo", "deep"])), "NODE-L3");
        assert!(get_node(&data, &rel(&["01.Math", "PJ.Demo", "deep", "deeper"])).is_none());
        assert!(data.get(".ant-trash").is_none());
        assert!(!sync(dir.path(), &mut data).unwrap());
    }

    #[test]
    fn sync_branch_lists_only_the_levels_down_to_the_folder() {
        let dir = TempDir::new("sync-branch");
        dir.mkdir("01.Math/01.Alg/01.Deep");
        dir.mkdir("02.Bio/01.Cell");
        let mut data = parse("01.Math: {type: NODE-L1}\n");
        assert!(sync_branch(dir.path(), &mut data, &rel(&["01.Math"])).unwrap());
        assert_eq!(get_type(&data, &rel(&["02.Bio"])), "NODE-L1");
        assert_eq!(get_type(&data, &rel(&["01.Math", "01.Alg"])), "NODE-L2");
        assert!(get_node(&data, &rel(&["02.Bio", "01.Cell"])).is_none());
        assert!(get_node(&data, &rel(&["01.Math", "01.Alg", "01.Deep"])).is_none());
        assert!(!sync_branch(dir.path(), &mut data, &rel(&["01.Math"])).unwrap());
    }

    #[test]
    fn infer_type_follows_prefixes_and_depth() {
        assert_eq!(infer_type("PJ.Thesis", 1), "WKB");
        assert_eq!(infer_type("PR.X", 0), "WKB");
        assert_eq!(infer_type("HW.X", 2), "WKB");
        assert_eq!(infer_type("90.Resources", 1), "RES");
        assert_eq!(infer_type("97.Templates", 1), "TPL");
        assert_eq!(infer_type("98.Review", 2), "REV");
        assert_eq!(infer_type("01.Math", 0), "NODE-L1");
        assert_eq!(infer_type("a/b/02.X", 2), "NODE-L3");
    }

    #[test]
    fn locked_types_are_rev_res_tpl_cfg() {
        for t in ["REV", "REV-L3", "RES", "TPL-L1", "CFG"] {
            assert!(is_locked_type(t), "{t}");
        }
        for t in ["NODE-L1", "WKB", "PORTF", "", "REVX"] {
            assert!(!is_locked_type(t), "{t}");
        }
    }

    #[test]
    fn rename_and_delete_key_act_on_the_parent_mapping() {
        let mut data = parse("p:\n  a: {type: X}\n  b: {type: Y}\n");
        assert!(rename_key(&mut data, &rel(&["p"]), "a", "c"));
        assert_eq!(get_type(&data, &rel(&["p", "c"])), "X");
        assert!(get_node(&data, &rel(&["p", "a"])).is_none());
        assert!(!rename_key(&mut data, &rel(&["p"]), "nope", "d"));
        assert!(!rename_key(&mut data, &rel(&["missing"]), "a", "d"));
        assert!(delete_key(&mut data, &rel(&["p"]), "b"));
        assert!(!delete_key(&mut data, &rel(&["p"]), "b"));
    }

    #[test]
    fn rename_and_delete_key_never_touch_meta_values() {
        let mut data = parse("p:\n  type: NODE-L1\n  status: done\n  a: {type: X}\n");
        assert!(!delete_key(&mut data, &rel(&["p"]), "type"));
        assert!(!rename_key(&mut data, &rel(&["p"]), "status", "x"));
        assert!(!rename_key(&mut data, &rel(&["p"]), "a", "type"));
        assert_eq!(get_type(&data, &rel(&["p"])), "NODE-L1");
        assert_eq!(get_type(&data, &rel(&["p", "a"])), "X");
        assert!(get_node(&data, &rel(&["p"])).unwrap().get("status").is_some());
    }

    const RICH: &str = "version: 3\nzeta:\n  type: NODE-L1\n  status: done\n  note: hello\n  a: {type: Y}\n  b: {}\nalpha:\n  type: REV\n";

    #[cfg(windows)]
    #[test]
    fn save_creates_a_hidden_db() {
        use std::os::windows::fs::MetadataExt;
        let dir = TempDir::new("db-created");
        save(dir.path(), &parse(RICH)).unwrap();
        let data_dir = dir.path().join(".ant");
        assert!(data_dir.join("ant.db").is_file());
        assert_ne!(std::fs::metadata(&data_dir).unwrap().file_attributes() & 0x2, 0, ".ant is not hidden");
    }

    #[test]
    fn db_round_trips_extra_keys_and_root_entries() {
        let dir = TempDir::new("db-roundtrip");
        let data = sort_mapping(&parse(RICH));
        save(dir.path(), &data).unwrap();
        assert_eq!(load(dir.path()).unwrap(), data);
    }

    #[cfg(windows)]
    #[test]
    fn a_failed_db_write_leaves_the_tree_unchanged() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = TempDir::new("db-failed-write");
        let before = parse("one: {type: NODE-L1}\n");
        save(dir.path(), &before).unwrap();
        let _held = std::fs::OpenOptions::new().read(true).share_mode(0x1).open(crate::storage::db::db_path(dir.path())).unwrap();
        assert!(save(dir.path(), &parse("two: {type: NODE-L1}\n")).is_err());
        assert_eq!(load(dir.path()).unwrap(), before);
    }

    #[test]
    fn only_a_new_storage_gives_level_three_nodes_the_data_flag() {
        let dir = TempDir::new("fresh-l3");
        dir.mkdir("100.A/01.B/01.C");
        let mut data = Mapping::new();
        sync(dir.path(), &mut data).unwrap();
        assert_eq!(get_node(&data, &rel(&["100.A", "01.B", "01.C"])).unwrap().get("flags").and_then(Value::as_str), Some("data"));
        assert!(get_node(&data, &rel(&["100.A", "01.B"])).unwrap().get("flags").is_none());
        save(dir.path(), &data).unwrap();
        dir.mkdir("100.A/01.B/02.D");
        sync(dir.path(), &mut data).unwrap();
        assert!(get_node(&data, &rel(&["100.A", "01.B", "02.D"])).unwrap().get("flags").is_none());
    }
}
