// Tree access on ant.db: load, save, sync with the folders on disk and edit entries by path.
#[cfg(test)]
mod tests;

use std::path::Path;

use serde_yaml::{Mapping, Value};

use crate::domain::tree_types::{flags_in, type_base, META_KEYS};
use crate::domain::config::{DATA_FLAG, HIDDEN_FLAG, LOCKED_TYPES, PORTFOLIO_FOLDER, WKB_IDENTIFIERS};
use crate::infra::storage::db::{db_path, Store};
use crate::domain::error::R;
use crate::infra::fs_safe::{io_err, join_rel};
use crate::infra::footprint_file;
use crate::domain::names::{is_system_root_name, support_kind};
use crate::infra::scan::list_subdirs;
use crate::services::tree::depth;

pub fn load(root: &Path) -> R<Mapping> {
    match Store::open_existing(root)? {
        Some(store) => store.read(),
        None => Ok(Mapping::new()),
    }
}

// Only the path down to `rel` and what lies under it; the rest of the tree stays unread.
pub fn load_level(root: &Path, rel: &[String]) -> R<Mapping> {
    match Store::open_existing(root)? {
        Some(store) => store.read_level(rel),
        None => Ok(Mapping::new()),
    }
}

// Each path with its ancestors and no children, for lookups of a node's own entry.
pub fn load_paths(root: &Path, rels: &[Vec<String>]) -> R<Mapping> {
    match Store::open_existing(root)? {
        Some(store) => store.read_paths(rels),
        None => Ok(Mapping::new()),
    }
}

pub fn save(root: &Path, data: &Mapping) -> R<()> {
    Store::open_or_create(root)?.replace(&sort_mapping(data))
}

// Writes only the subtrees at `touched`, for a change known to stay inside them; the rest of the db is not compared.
pub fn save_paths(root: &Path, data: &Mapping, touched: &[Vec<String>]) -> R<()> {
    Store::open_or_create(root)?.replace_paths(&sort_mapping(data), touched)
}

// Folders added on disk since the last refresh get their tree entry before any check runs.
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
    match support_kind(basename) {
        Some(kind) => kind.to_string(),
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
    let (renamed, paths) = sync_branch_paths(root, data, dir)?;
    Ok(renamed || !paths.is_empty())
}

// Also names the folders whose entries changed; the flag is a legacy rename, which can touch rows outside them.
pub fn sync_branch_paths(root: &Path, data: &mut Mapping, dir: &[String]) -> R<(bool, Vec<Vec<String>>)> {
    let fresh = !db_path(root).is_file();
    let limit = depth::limit(root)?;
    let renamed = rename_legacy_types(data);
    let (mut node, mut path, mut changed) = (data, root.to_path_buf(), Vec::new());
    for depth in 0..=dir.len().min(limit - 1) {
        let names = list_subdirs(&path)?;
        for name in names.iter().filter(|n| !META_KEYS.contains(&n.as_str())) {
            let mut touched = add_missing(node, name, depth, fresh);
            if let Some(Value::Mapping(child)) = node.get_mut(name.as_str()) {
                touched |= reconcile_hidden(&path.join(name), child)?;
            }
            if touched {
                let mut rel = dir[..depth.min(dir.len())].to_vec();
                rel.push(name.clone());
                changed.push(rel);
            }
        }
        // Exact names only, as a full sync only descends into folders listed on disk.
        let Some(part) = dir.get(depth).filter(|p| names.contains(p)) else { break };
        let Some(Value::Mapping(child)) = node.get_mut(part.as_str()) else { break };
        node = child;
        path.push(part);
    }
    Ok((renamed, changed))
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
