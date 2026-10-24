// Installed extensions: the folder scan, their on/off state in ant.db and removal.
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::fail;
use super::manifest::{self, Manifest, MAX_JSON_BYTES, MAX_MAIN_BYTES};
use crate::config::DATA_DIR;
use crate::error::R;
use crate::fs_safe::{io_err, is_link};
use crate::storage::db::Store;

const EXTENSIONS_DIR: &str = "extensions";
const STATE_KEY: &str = "extensions";
pub const SOURCE_FILE: &str = ".source.json";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub permissions: Vec<String>,
    pub trust: &'static str,
    pub enabled: bool,
    pub status: &'static str,
    pub reason: Option<String>,
}

#[derive(Deserialize)]
struct StateEntry {
    enabled: bool,
}

#[derive(Deserialize)]
struct Source {
    trust: String,
}

pub fn extensions_dir(root: &Path) -> PathBuf {
    root.join(DATA_DIR).join(EXTENSIONS_DIR)
}

// The backend is the only writer, so an unreadable value just leaves every extension off.
pub fn parse_state(value: &str) -> BTreeMap<String, bool> {
    serde_yaml::from_str::<BTreeMap<String, StateEntry>>(value)
        .map(|m| m.into_iter().filter(|(id, _)| manifest::valid_id(id)).map(|(id, e)| (id, e.enabled)).collect())
        .unwrap_or_default()
}

// Ids are plain [a-z0-9-], so they need no JSON escaping.
pub fn state_json(state: &BTreeMap<String, bool>) -> String {
    let body: Vec<String> = state.iter().map(|(id, on)| format!("\"{id}\":{{\"enabled\":{on}}}")).collect();
    format!("{{{}}}", body.join(","))
}

pub fn prune(state: &mut BTreeMap<String, bool>, present: &HashSet<&str>) -> bool {
    let before = state.len();
    state.retain(|id, _| present.contains(id.as_str()));
    state.len() != before
}

fn read_state(root: &Path) -> R<BTreeMap<String, bool>> {
    let Some(store) = Store::open_existing(root)? else { return Ok(BTreeMap::new()) };
    Ok(store.settings()?.get(STATE_KEY).map(|v| parse_state(v)).unwrap_or_default())
}

fn write_state(root: &Path, state: &BTreeMap<String, bool>) -> R<()> {
    Store::open_or_create(root)?.set_setting(STATE_KEY, &state_json(state))
}

pub fn forget(root: &Path, id: &str) -> R<()> {
    let mut state = read_state(root)?;
    if state.remove(id).is_some() {
        write_state(root, &state)?;
    }
    Ok(())
}

fn trust(dir: &Path) -> &'static str {
    let source = manifest::read_small(&dir.join(SOURCE_FILE), MAX_JSON_BYTES)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .and_then(|text| serde_yaml::from_str::<Source>(&text).ok());
    match source {
        Some(s) if s.trust == "validated" => "validated",
        _ => "unvalidated",
    }
}

fn checked(dir: &Path, folder: &str) -> Result<Manifest, String> {
    let manifest = manifest::load(dir)?;
    if manifest.id != folder {
        return Err(format!("folder name {folder:?} differs from the id {:?}", manifest.id));
    }
    Ok(manifest)
}

pub fn info(dir: &Path, folder: &str, enabled: bool) -> ExtensionInfo {
    let trust = trust(dir);
    match checked(dir, folder) {
        Ok(m) => ExtensionInfo {
            id: m.id,
            name: m.name,
            version: m.version,
            description: m.description,
            author: m.author,
            permissions: m.permissions,
            trust,
            enabled,
            status: "ok",
            reason: None,
        },
        Err(reason) => ExtensionInfo {
            id: folder.to_string(),
            name: folder.to_string(),
            version: String::new(),
            description: String::new(),
            author: String::new(),
            permissions: Vec::new(),
            trust,
            enabled: false,
            status: "rejected",
            reason: Some(reason),
        },
    }
}

fn child_dirs(dir: &Path) -> R<Vec<(String, PathBuf)>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| io_err("read folder", dir, &e))? {
        let entry = entry.map_err(|e| io_err("read folder", dir, &e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let plain_dir = fs::symlink_metadata(entry.path()).is_ok_and(|m| m.is_dir() && !is_link(&m));
        if plain_dir && !name.starts_with('.') {
            out.push((name, entry.path()));
        }
    }
    Ok(out)
}

pub fn list(root: &Path) -> R<Vec<ExtensionInfo>> {
    let dirs = child_dirs(&extensions_dir(root))?;
    let mut state = read_state(root)?;
    if prune(&mut state, &dirs.iter().map(|(n, _)| n.as_str()).collect()) {
        write_state(root, &state)?;
    }
    let mut out: Vec<ExtensionInfo> =
        dirs.iter().map(|(name, dir)| info(dir, name, state.get(name).copied().unwrap_or(false))).collect();
    out.sort_by_key(|e| e.name.to_lowercase());
    Ok(out)
}

pub fn exists(root: &Path, id: &str) -> bool {
    fs::symlink_metadata(extensions_dir(root).join(id)).is_ok()
}

fn installed_dir(root: &Path, id: &str) -> R<PathBuf> {
    if !manifest::valid_id(id) {
        return Err(fail(format!("Invalid extension id {id:?}")));
    }
    let dir = extensions_dir(root).join(id);
    if !fs::symlink_metadata(&dir).is_ok_and(|m| m.is_dir() && !is_link(&m)) {
        return Err(fail(format!("Extension {id} is not installed")));
    }
    Ok(dir)
}

fn find_ok(root: &Path, id: &str) -> R<(PathBuf, Manifest)> {
    let dir = installed_dir(root, id)?;
    let manifest = checked(&dir, id).map_err(|reason| fail(format!("Extension {id} is rejected: {reason}")))?;
    Ok((dir, manifest))
}

pub fn set_enabled(root: &Path, id: &str, enabled: bool) -> R<()> {
    find_ok(root, id)?;
    let mut state = read_state(root)?;
    state.insert(id.to_string(), enabled);
    write_state(root, &state)
}

pub fn read_entry(root: &Path, id: &str) -> R<String> {
    let (dir, manifest) = find_ok(root, id)?;
    if !read_state(root)?.get(id).copied().unwrap_or(false) {
        return Err(fail(format!("Extension {id} is disabled")));
    }
    let file = manifest::main_file(&dir, &manifest.main).map_err(fail)?;
    let bytes = manifest::read_small(&file, MAX_MAIN_BYTES).map_err(fail)?;
    String::from_utf8(bytes).map_err(|_| fail(format!("{} is not UTF-8", manifest.main)))
}

pub fn remove(root: &Path, id: &str) -> R<()> {
    let dir = installed_dir(root, id)?;
    fs::remove_dir_all(&dir).map_err(|e| io_err("delete folder", &dir, &e))?;
    forget(root, id)
}
