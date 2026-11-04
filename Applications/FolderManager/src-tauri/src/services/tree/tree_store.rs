// Tree access on ant.db: load, save, sync with the folders on disk and edit entries by path.
use std::path::Path;

use serde_yaml::{Mapping, Value};

use crate::domain::tree_types::{flags_in, type_base, META_KEYS};
use crate::domain::config::{DATA_FLAG, HIDDEN_FLAG, LOCKED_TYPES, PORTFOLIO_FOLDER, SPECIAL_TYPES, WKB_IDENTIFIERS};
use crate::infra::storage::db::{db_path, Store};
use crate::domain::error::R;
use crate::infra::fs_safe::{io_err, join_rel};
use crate::infra::footprint_file;
use crate::domain::names::is_system_root_name;
use crate::infra::scan::list_subdirs;
use crate::services::tree::depth;

pub fn load(root: &Path) -> R<Mapping> {
    match Store::open_existing(root)? {
        Some(store) => store.read(),
        None => Ok(Mapping::new()),
    }
}

pub fn save(root: &Path, data: &Mapping) -> R<()> {
    Store::open_or_create(root)?.replace(&sort_mapping(data))
}

// Folders added on disk since the last refresh get their yaml entry before any check runs.
pub(crate) fn load_synced(root: &Path) -> R<(Mapping, bool)> {
    let mut data = load(root)?;
    let changed = sync(root, &mut data)?;
    Ok((data, changed))
}

pub fn infer_type(name: &str, depth: usize) -> String {
    let basename = name.rsplit(['/', '\\']).next().unwrap_or(name);
    if basename.starts_with("00.") {
        return "OUT".to_string();
    }
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
    let renamed = rename_legacy_types(data);
    Ok(renamed | sync_level(root, data, 0, fresh, depth::limit(root)?)?)
}

// Storages saved before the Portfolio type was named OUT still store PORTF; they are rewritten as OUT.
fn rename_legacy_types(node: &mut Mapping) -> bool {
    let mut changed = false;
    for (_, value) in node.iter_mut() {
        let Value::Mapping(child) = value else { continue };
        if let Some(Value::String(kind)) = child.get_mut("type") {
            if kind.starts_with("PORTF") {
                *kind = kind.replacen("PORTF", "OUT", 1);
                changed = true;
            }
        }
        changed |= rename_legacy_types(child);
    }
    changed
}

pub(crate) fn set_flag(node: &mut Mapping, flag: &str, on: bool) {
    let mut flags = flags_in(node);
    flags.retain(|f| f != flag);
    if on {
        flags.push(flag.to_string());
    }
    if flags.is_empty() {
        node.remove("flags");
    } else {
        node.insert("flags".into(), flags.join(",").into());
    }
}

// The database and the folder's dragpath.json agree on "hidden": a true on either side hides the folder, and a missing key never unhides it.
fn reconcile_hidden(folder: &Path, node: &mut Mapping) -> R<bool> {
    let on_disk = footprint_file::read_hidden(folder);
    let in_db = flags_in(node).iter().any(|f| f == HIDDEN_FLAG);
    match (on_disk, in_db) {
        (true, false) => {
            set_flag(node, HIDDEN_FLAG, true);
            Ok(true)
        }
        (false, true) => {
            footprint_file::mirror_hidden(folder, true)?;
            Ok(false)
        }
        _ => Ok(false),
    }
}

// Portfolios are reconciled apart from the sync, so a sync or a mutation never creates or removes a 00. folder on its own.
pub fn reconcile_portfolios_all(root: &Path, data: &mut Mapping) -> R<bool> {
    reconcile_level(root, data, 0, depth::limit(root)?)
}

fn reconcile_level(dir: &Path, node: &mut Mapping, depth: usize, limit: usize) -> R<bool> {
    if depth >= limit {
        return Ok(false);
    }
    let mut changed = false;
    let names: Vec<String> = node.keys().filter_map(Value::as_str).map(str::to_string).collect();
    for name in names {
        // The app's 9xx root folders hold no workboards, so nothing beneath them gets a portfolio.
        if depth == 0 && is_system_root_name(&name) {
            continue;
        }
        if let Some(Value::Mapping(child)) = node.get_mut(name.as_str()) {
            if !flags_in(child).iter().any(|f| f == HIDDEN_FLAG) {
                changed |= reconcile_level(&dir.join(&name), child, depth + 1, limit)?;
            }
        }
    }
    if depth > 0 {
        changed |= reconcile_portfolios(dir, node, depth)?;
    }
    Ok(changed)
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
    let limit = depth::limit(root)?;
    let renamed = rename_legacy_types(data);
    let (mut node, mut path, mut changed) = (data, root.to_path_buf(), renamed);
    for depth in 0..=dir.len().min(limit - 1) {
        let names = list_subdirs(&path)?;
        for name in names.iter().filter(|n| !META_KEYS.contains(&n.as_str())) {
            changed |= add_missing(node, name, depth, fresh);
            if let Some(Value::Mapping(child)) = node.get_mut(name.as_str()) {
                changed |= reconcile_hidden(&path.join(name), child)?;
            }
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
    sync_level(&dir, node, rel.len(), !db_path(root).is_file(), depth::limit(root)?)
}

fn sync_level(dir: &Path, node: &mut Mapping, depth: usize, fresh: bool, limit: usize) -> R<bool> {
    if depth >= limit {
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
            changed |= reconcile_hidden(&dir.join(&name), child)?;
            if !flags_in(child).iter().any(|f| f == HIDDEN_FLAG) {
                changed |= sync_level(&dir.join(&name), child, depth + 1, fresh, limit)?;
            }
        }
    }
    Ok(changed)
}

// A 00. Portfolio sits beside the workboards of its level: it appears with the first one, and goes once none is left and it is empty.
fn reconcile_portfolios(dir: &Path, node: &mut Mapping, depth: usize) -> R<bool> {
    let has_workboard = node.iter().any(|(name, entry)| {
        name.as_str().is_some_and(|n| dir.join(n).is_dir())
            && entry.get("type").and_then(Value::as_str).is_some_and(|t| type_base(t) == "WKB")
    });
    let portfolios: Vec<String> = node.keys().filter_map(Value::as_str).filter(|n| n.starts_with("00.")).map(str::to_string).collect();
    let mut changed = false;
    for name in &portfolios {
        let Some(Value::Mapping(entry)) = node.get_mut(name.as_str()) else { continue };
        if entry.get("type").and_then(Value::as_str).is_some_and(|t| type_base(t) == "NODE") {
            entry.insert("type".into(), format!("OUT-L{}", depth + 1).into());
            changed = true;
        }
    }
    if has_workboard {
        if portfolios.iter().any(|name| dir.join(name).is_dir()) || dir.join(PORTFOLIO_FOLDER).exists() {
            return Ok(changed);
        }
        let path = dir.join(PORTFOLIO_FOLDER);
        std::fs::create_dir(&path).map_err(|e| io_err("create portfolio", &path, &e))?;
        let mut entry = Mapping::new();
        entry.insert("type".into(), "OUT".into());
        node.insert(PORTFOLIO_FOLDER.into(), Value::Mapping(entry));
        return Ok(true);
    }
    for name in &portfolios {
        let path = dir.join(name);
        if path.is_dir() {
            if !is_empty_dir(&path)? {
                continue;
            }
            std::fs::remove_dir(&path).map_err(|e| io_err("remove portfolio", &path, &e))?;
        }
        node.remove(name.as_str());
        changed = true;
    }
    Ok(changed)
}

fn is_empty_dir(path: &Path) -> R<bool> {
    let mut entries = std::fs::read_dir(path).map_err(|e| io_err("list folder", path, &e))?;
    Ok(entries.next().is_none())
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

    fn settle(root: &Path, data: &mut Mapping) -> bool {
        sync(root, data).unwrap() | reconcile_portfolios_all(root, data).unwrap()
    }

    #[test]
    fn a_portfolio_follows_its_workboards_on_disk() {
        let dir = TempDir::new("portfolio-follow");
        dir.mkdir("100.A/01.Plan/PJ.001_Thesis");
        let mut data = parse("");
        settle(dir.path(), &mut data);
        assert!(dir.path().join("100.A/01.Plan/00.Outcomes").is_dir());
        assert_eq!(get_type(&data, &rel(&["100.A", "01.Plan", "00.Outcomes"])), "OUT");
        std::fs::remove_dir_all(dir.path().join("100.A/01.Plan/PJ.001_Thesis")).unwrap();
        assert!(settle(dir.path(), &mut data));
        assert!(!dir.path().join("100.A/01.Plan/00.Outcomes").exists());
        assert!(get_node(&data, &rel(&["100.A", "01.Plan", "00.Outcomes"])).is_none());
    }

    #[test]
    fn a_portfolio_with_content_stays_once_its_workboards_are_gone() {
        let dir = TempDir::new("portfolio-content");
        dir.mkdir("100.A/01.Plan/00.Porfolio/01.Notes");
        let mut data = parse("");
        settle(dir.path(), &mut data);
        assert_eq!(get_type(&data, &rel(&["100.A", "01.Plan", "00.Porfolio"])), "OUT");
        assert!(!dir.path().join("100.A/01.Plan/00.Outcomes").exists());
        assert!(dir.path().join("100.A/01.Plan/00.Porfolio/01.Notes").is_dir());
    }

    #[test]
    fn a_legacy_00_node_becomes_a_portfolio_without_a_rename() {
        let dir = TempDir::new("portfolio-legacy");
        dir.mkdir("100.A/00.Porfolio/01.Notes");
        let mut data = parse("100.A:\n  type: NODE-L1\n  00.Porfolio: {type: NODE-L2}\n");
        assert!(settle(dir.path(), &mut data));
        assert_eq!(get_type(&data, &rel(&["100.A", "00.Porfolio"])), "OUT-L2");
        assert!(dir.path().join("100.A/00.Porfolio").is_dir());
    }

    #[test]
    fn the_system_folders_get_no_portfolio() {
        let dir = TempDir::new("portfolio-system");
        dir.mkdir("999.Settings/00.Core");
        dir.mkdir("999.Settings/PJ.001_Inside");
        let mut data = parse("");
        settle(dir.path(), &mut data);
        assert!(!dir.path().join("999.Settings/00.Outcomes").exists());
        assert!(dir.path().join("999.Settings/00.Core").is_dir());
    }

    #[test]
    fn the_hidden_key_in_dragpath_and_the_database_agree_after_a_sync() {
        let dir = TempDir::new("dragpath-hidden");
        dir.mkdir("100.A/02.Bio/.ant");
        dir.mkdir("100.A/03.Art/.ant");
        std::fs::write(dir.path().join("100.A/02.Bio/.ant/dragpath.json"), "{\"id\": \"bio-2\", \"hidden\": true}\n").unwrap();
        std::fs::write(dir.path().join("100.A/03.Art/.ant/dragpath.json"), "{\"id\": \"art-3\"}\n").unwrap();
        let mut data = parse("100.A:\n  type: NODE-L1\n  03.Art:\n    type: NODE-L2\n    flags: hidden\n");
        settle(dir.path(), &mut data);
        assert!(flags_in(get_node(&data, &rel(&["100.A", "02.Bio"])).unwrap()).contains(&HIDDEN_FLAG.to_string()));
        assert!(!flags_in(get_node(&data, &rel(&["100.A", "03.Art"])).unwrap()).is_empty());
        assert!(std::fs::read_to_string(dir.path().join("100.A/03.Art/.ant/dragpath.json")).unwrap().contains("\"hidden\": true"));
    }

    #[test]
    fn a_legacy_portf_type_is_read_and_rewritten_as_out() {
        let mut data = parse("100.A:\n  type: NODE-L1\n  02.B: {type: PORTF-L2}\n  00.P: {type: PORTF}\n");
        assert!(rename_legacy_types(&mut data));
        assert_eq!(get_type(&data, &rel(&["100.A", "02.B"])), "OUT-L2");
        assert_eq!(get_type(&data, &rel(&["100.A", "00.P"])), "OUT");
        assert!(!rename_legacy_types(&mut data));
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
        assert_eq!(infer_type("00.Porfolio", 1), "OUT");
        assert_eq!(infer_type("01.Math", 0), "NODE-L1");
        assert_eq!(infer_type("a/b/02.X", 2), "NODE-L3");
    }

    #[test]
    fn locked_types_are_rev_res_tpl_cfg() {
        for t in ["REV", "REV-L3", "RES", "TPL-L1", "CFG"] {
            assert!(is_locked_type(t), "{t}");
        }
        for t in ["NODE-L1", "WKB", "OUT", "", "REVX"] {
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
        let _held = std::fs::OpenOptions::new().read(true).share_mode(0x1).open(crate::infra::storage::db::db_path(dir.path())).unwrap();
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
