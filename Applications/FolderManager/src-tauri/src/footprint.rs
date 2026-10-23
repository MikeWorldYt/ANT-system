// Gives every folder that has a .ant folder a lasting id, so moves made outside the app are recognised.
use std::collections::hash_map::RandomState;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{BuildHasher, Hasher};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_yaml::{Mapping, Value};

use crate::ant_core::tree_types::META_KEYS;
use crate::config::{DATA_DIR, FOOTPRINT_FILENAME, MAX_DEPTH};
use crate::db::Store;
use crate::error::R;
use crate::fs_safe::{join_rel, write_atomic};
use crate::names::list_subdirs;
use crate::tree_node::is_reserved;
use crate::tree_store::{delete_key, get_node, get_node_mut};

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const ID_LEN: usize = 15;

struct Live {
    rel: Vec<String>,
    id: Option<String>,
}

pub struct Scan {
    live: Vec<Live>,
    known: HashMap<String, String>,
}

impl Scan {
    // Clean means every folder already has an id and the db remembers exactly these paths.
    pub fn is_clean(&self) -> bool {
        self.live.len() == self.known.len()
            && self.live.iter().all(|l| l.id.as_ref().is_some_and(|id| self.known.get(id) == Some(&l.rel.join("/"))))
    }
}

pub fn scan(root: &Path) -> R<Scan> {
    let mut live = Vec::new();
    walk(&mut Vec::new(), root, &mut live)?;
    let known = match Store::open_existing(root)? {
        Some(store) => store.footprints()?,
        None => HashMap::new(),
    };
    Ok(Scan { live, known })
}

// What apply decided: whether the yaml changed and the id -> path map to persist once that change is saved.
pub struct Applied {
    pub changed: bool,
    remembered: HashMap<String, String>,
}

impl Applied {
    pub fn commit(&self, root: &Path) -> R<()> {
        Store::open_or_create(root)?.replace_footprints(&self.remembered)
    }
}

// Writes missing ids and carries moved folders' yaml entries to their new place; persisting the map is left to commit.
pub fn plan(root: &Path, data: &mut Mapping) -> R<Applied> {
    let Scan { live, known } = scan(root)?;
    let mut taken: HashSet<String> = known.keys().cloned().chain(live.iter().filter_map(|l| l.id.clone())).collect();
    let mut by_id: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    for l in live {
        let id = match l.id {
            Some(id) => id,
            None => {
                let id = new_id(&taken);
                write_id(&join_rel(root, &l.rel), &id)?;
                taken.insert(id.clone());
                id
            }
        };
        by_id.entry(id).or_default().push(l.rel);
    }

    let mut moves: Vec<(Vec<String>, Vec<String>)> = Vec::new();
    let mut remembered = HashMap::new();
    for (id, paths) in by_id {
        let current = match paths.as_slice() {
            [only] => {
                // can you find me?
                // A folder moved outside the app is found again by its id instead of showing as a ghost plus a new folder.
                if let Some(old) = known.get(&id).filter(|old| **old != only.join("/")) {
                    let old_rel: Vec<String> = old.split('/').map(str::to_string).collect();
                    if !still_listed(root, &old_rel) {
                        moves.push((old_rel, only.clone()));
                    }
                }
                only.join("/")
            }
            // Copies share an id; the path the db already knows keeps it.
            _ => known.get(&id).filter(|k| paths.iter().any(|p| p.join("/") == **k)).cloned().unwrap_or_else(|| paths[0].join("/")),
        };
        remembered.insert(id, current);
    }

    // Shallowest first, so a renamed parent carries its subtree before the children are re-keyed inside it.
    moves.sort_by_key(|(old, _)| old.len());
    let mut done: Vec<(Vec<String>, Vec<String>)> = Vec::new();
    let mut changed = false;
    for (old, new) in moves {
        let mut from = old;
        for (done_old, done_new) in &done {
            if from.starts_with(done_old) {
                from.splice(..done_old.len(), done_new.iter().cloned());
                break;
            }
        }
        if from != new && migrate(data, &from, &new) {
            changed = true;
        }
        done.push((from, new));
    }
    Ok(Applied { changed, remembered })
}

#[cfg(test)]
pub fn apply(root: &Path, data: &mut Mapping) -> R<bool> {
    let applied = plan(root, data)?;
    applied.commit(root)?;
    Ok(applied.changed)
}

// Compares the real directory listing, because on Windows a case-only rename still resolves under the old name.
fn still_listed(root: &Path, rel: &[String]) -> bool {
    let Some((name, parent)) = rel.split_last() else { return false };
    let dir = join_rel(root, parent);
    dir.is_dir() && list_subdirs(&dir).is_ok_and(|names| names.contains(name))
}

fn walk(rel: &mut Vec<String>, dir: &Path, out: &mut Vec<Live>) -> R<()> {
    if rel.len() >= MAX_DEPTH {
        return Ok(());
    }
    for name in list_subdirs(dir)? {
        if META_KEYS.contains(&name.as_str()) || (rel.is_empty() && is_reserved(std::slice::from_ref(&name))) {
            continue;
        }
        let folder = dir.join(&name);
        rel.push(name);
        if folder.join(DATA_DIR).is_dir() {
            out.push(Live { rel: rel.clone(), id: read_id(&folder) });
        }
        walk(rel, &folder, out)?;
        rel.pop();
    }
    Ok(())
}

// An unreadable or malformed file counts as no id, so a new one replaces it.
fn read_id(folder: &Path) -> Option<String> {
    let text = fs::read_to_string(folder.join(DATA_DIR).join(FOOTPRINT_FILENAME)).ok()?;
    match serde_yaml::from_str::<Value>(&text).ok()? {
        Value::Mapping(m) => m.get("id").and_then(Value::as_str).filter(|id| !id.is_empty() && !id.contains(char::is_whitespace)).map(str::to_string),
        _ => None,
    }
}

fn write_id(folder: &Path, id: &str) -> R<()> {
    write_atomic(&folder.join(DATA_DIR).join(FOOTPRINT_FILENAME), format!("{{\"id\": \"{id}\"}}\n").as_bytes())
}

fn migrate(data: &mut Mapping, old: &[String], new: &[String]) -> bool {
    let (Some((old_name, old_parent)), Some((new_name, new_parent))) = (old.split_last(), new.split_last()) else {
        return false;
    };
    let Some(entry) = get_node(data, old).cloned() else { return false };
    if !new_parent.is_empty() && get_node(data, new_parent).is_none() {
        return false;
    }
    delete_key(data, old_parent, old_name);
    let slot = if new_parent.is_empty() { Some(data) } else { get_node_mut(data, new_parent) };
    match slot {
        Some(slot) => {
            slot.insert(Value::String(new_name.clone()), Value::Mapping(entry));
            true
        }
        None => false,
    }
}

fn new_id(taken: &HashSet<String>) -> String {
    loop {
        let id = random_id();
        if !taken.contains(&id) {
            return id;
        }
    }
}

fn random_id() -> String {
    let words = [random_word(1), random_word(2)];
    (0..ID_LEN).map(|i| ALPHABET[((words[i / 12] >> ((i % 12) * 5)) & 31) as usize] as char).collect()
}

// RandomState is seeded from the OS, so this needs no crate and ids differ between runs.
fn random_word(salt: u64) -> u64 {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(salt);
    hasher.write_u128(SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
    hasher.write_u32(std::process::id());
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree_store::test_support::TempDir;
    use crate::tree_store::sync;

    fn rel(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    fn storage() -> (TempDir, Mapping) {
        let dir = TempDir::new("footprint");
        dir.mkdir("100.A/PJ.001/.ant");
        let mut data: Mapping = serde_yaml::from_str("100.A:\n  type: NODE-L1\n  PJ.001: {type: WKB-L2, status: hold}\n").unwrap();
        sync(dir.path(), &mut data).unwrap();
        (dir, data)
    }

    #[test]
    fn ids_are_fifteen_characters_from_the_alphabet_and_avoid_taken_ones() {
        let mut taken = HashSet::new();
        for _ in 0..500 {
            let id = new_id(&taken);
            assert_eq!(id.len(), ID_LEN);
            assert!(id.bytes().all(|b| ALPHABET.contains(&b)));
            assert!(taken.insert(id));
        }
    }

    #[test]
    fn a_folder_with_a_dot_ant_gets_one_id_and_keeps_it() {
        let (dir, mut data) = storage();
        assert!(!scan(dir.path()).unwrap().is_clean());
        assert!(!apply(dir.path(), &mut data).unwrap());
        let first = read_id(&dir.path().join("100.A/PJ.001")).unwrap();
        assert_eq!(first.len(), ID_LEN);
        assert!(scan(dir.path()).unwrap().is_clean());
        apply(dir.path(), &mut data).unwrap();
        assert_eq!(read_id(&dir.path().join("100.A/PJ.001")).unwrap(), first);
    }

    #[test]
    fn a_folder_without_dot_ant_is_left_alone() {
        let dir = TempDir::new("footprint-plain");
        dir.mkdir("100.A/PJ.001");
        let mut data = Mapping::new();
        apply(dir.path(), &mut data).unwrap();
        assert!(!dir.path().join("100.A/PJ.001/.ant").exists());
    }

    #[test]
    fn a_folder_moved_outside_the_app_keeps_its_type_and_status() {
        let (dir, mut data) = storage();
        apply(dir.path(), &mut data).unwrap();
        fs::rename(dir.path().join("100.A/PJ.001"), dir.path().join("100.A/PJ.009")).unwrap();
        sync(dir.path(), &mut data).unwrap();
        assert!(apply(dir.path(), &mut data).unwrap());
        let moved = get_node(&data, &rel(&["100.A", "PJ.009"])).unwrap();
        assert_eq!(moved.get("status").and_then(Value::as_str), Some("hold"));
        assert!(get_node(&data, &rel(&["100.A", "PJ.001"])).is_none());
        assert!(scan(dir.path()).unwrap().is_clean());
    }

    #[test]
    fn a_copied_folder_is_not_taken_for_a_move() {
        let (dir, mut data) = storage();
        apply(dir.path(), &mut data).unwrap();
        dir.mkdir("100.A/PJ.002/.ant");
        fs::copy(dir.path().join("100.A/PJ.001/.ant/dragpath.json"), dir.path().join("100.A/PJ.002/.ant/dragpath.json")).unwrap();
        sync(dir.path(), &mut data).unwrap();
        assert!(!apply(dir.path(), &mut data).unwrap());
        assert!(get_node(&data, &rel(&["100.A", "PJ.001"])).is_some() && get_node(&data, &rel(&["100.A", "PJ.002"])).is_some());
    }

    #[test]
    fn a_case_only_rename_migrates_the_entry() {
        let (dir, mut data) = storage();
        apply(dir.path(), &mut data).unwrap();
        fs::rename(dir.path().join("100.A/PJ.001"), dir.path().join("100.A/pj.001")).unwrap();
        sync(dir.path(), &mut data).unwrap();
        assert!(apply(dir.path(), &mut data).unwrap());
        let moved = get_node(&data, &rel(&["100.A", "pj.001"])).unwrap();
        assert_eq!(moved.get("status").and_then(Value::as_str), Some("hold"));
    }

    #[test]
    fn a_renamed_parent_and_child_keep_the_child_entry() {
        let dir = TempDir::new("footprint-subtree");
        dir.mkdir("100.A/.ant");
        dir.mkdir("100.A/PJ.001/.ant");
        let mut data: Mapping = serde_yaml::from_str("100.A:
  type: NODE-L1
  PJ.001: {type: WKB-L2, status: hold}
").unwrap();
        sync(dir.path(), &mut data).unwrap();
        apply(dir.path(), &mut data).unwrap();
        fs::rename(dir.path().join("100.A"), dir.path().join("200.B")).unwrap();
        fs::rename(dir.path().join("200.B/PJ.001"), dir.path().join("200.B/PJ.002")).unwrap();
        sync(dir.path(), &mut data).unwrap();
        assert!(apply(dir.path(), &mut data).unwrap());
        let child = get_node(&data, &rel(&["200.B", "PJ.002"])).unwrap();
        assert_eq!(child.get("status").and_then(Value::as_str), Some("hold"));
        assert!(get_node(&data, &rel(&["100.A"])).is_none());
    }

    #[test]
    fn plan_does_not_touch_the_db_until_commit() {
        let (dir, mut data) = storage();
        let applied = plan(dir.path(), &mut data).unwrap();
        assert!(!scan(dir.path()).unwrap().is_clean());
        applied.commit(dir.path()).unwrap();
        assert!(scan(dir.path()).unwrap().is_clean());
    }
}
