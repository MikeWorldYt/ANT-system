// Gives every folder that has a .ant folder a lasting id, so moves made outside the app are recognised.
use std::collections::hash_map::RandomState;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{BuildHasher, Hasher};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_yaml::{Mapping, Value};

use crate::ant_core::tree_types::{child_entries, type_base, META_KEYS};
use crate::config::{DATA_DIR, FOOTPRINT_FILENAME};
use crate::error::{FmError, R};
use crate::fs_safe::{io_err, join_rel, write_atomic};
use crate::ant_core::names::list_subdirs;
use crate::storage::db::hide;
use crate::tree::depth;
use crate::tree::tree_node::is_reserved;
use crate::tree::tree_store::{delete_key, get_node, get_node_mut};

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const ID_LEN: usize = 15;

struct Live {
    rel: Vec<String>,
    id: Option<String>,
}

pub struct Scan {
    live: Vec<Live>,
    known: HashMap<String, String>,
    // Nodes that should have a .ant folder but do not yet.
    unmarked: Vec<String>,
}

impl Scan {
    // Clean means every folder that carries an id has the one the tree remembers for its path.
    pub fn is_clean(&self) -> bool {
        self.unmarked.is_empty() && self.live.iter().all(|l| l.id.as_ref().is_some_and(|id| self.known.get(id) == Some(&l.rel.join("/"))))
    }
}

pub fn scan(root: &Path, data: &Mapping) -> R<Scan> {
    let mut live = Vec::new();
    walk(&mut Vec::new(), root, &mut live, depth::limit(root)?)?;
    let mut nodes = Vec::new();
    visit(data, &mut Vec::new(), &mut nodes);
    Ok(Scan {
        live,
        known: known_ids(&nodes),
        unmarked: unmarked(root, &nodes)?,
    })
}

// Every node in the tree as its "a/b" path, parents before children.
fn visit<'a>(node: &'a Mapping, parent: &mut Vec<String>, out: &mut Vec<(String, &'a Mapping)>) {
    for (name, child) in child_entries(node) {
        parent.push(name.to_string());
        out.push((parent.join("/"), child));
        visit(child, parent, out);
        parent.pop();
    }
}

fn known_ids(nodes: &[(String, &Mapping)]) -> HashMap<String, String> {
    nodes.iter().filter_map(|(path, node)| Some((node.get("uid").and_then(Value::as_str)?.to_string(), path.clone()))).collect()
}

// Folders that should carry a .ant folder for their id: nodes, data nodes included, and workboards.
// A node with loose files waits for the user before it is marked, so intake can ask first.
fn unmarked(root: &Path, nodes: &[(String, &Mapping)]) -> R<Vec<String>> {
    let mut out = Vec::new();
    for (path, node) in nodes {
        if !matches!(node.get("type").and_then(Value::as_str).map(type_base), Some("NODE" | "WKB")) {
            continue;
        }
        let rel = rel_of(path);
        let dir = join_rel(root, &rel);
        if dir.is_dir() && !dir.join(DATA_DIR).exists() && crate::intake::waiting_files(&dir, &rel, node)? == 0 {
            out.push(path.clone());
        }
    }
    Ok(out)
}

fn create_markers(root: &Path, paths: &[String]) -> R<()> {
    for path in paths {
        let dir = join_rel(root, &rel_of(path)).join(DATA_DIR);
        fs::create_dir(&dir).map_err(|e| io_err("create info folder", &dir, &e))?;
        hide(&dir);
    }
    Ok(())
}

fn rel_of(path: &str) -> Vec<String> {
    path.split('/').map(str::to_string).collect()
}

// Only a refresh the user sees tails copies, because each tailed copy must reach the question exactly once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Copies {
    Ignore,
    Tail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Copied {
    pub rel_parts: Vec<String>,
    pub original: Vec<String>,
}

pub struct Plan {
    pub changed: bool,
    pub copies: Vec<Copied>,
}

// Writes missing ids to disk, carries moved folders' entries to their new place and stores each uid in the tree.
pub fn plan(root: &Path, data: &mut Mapping, mode: Copies) -> R<Plan> {
    let mut nodes = Vec::new();
    visit(data, &mut Vec::new(), &mut nodes);
    create_markers(root, &unmarked(root, &nodes)?)?;
    let Scan { live, known, .. } = scan(root, data)?;
    let mut taken: HashSet<String> = known.keys().cloned().chain(live.iter().filter_map(|l| l.id.clone())).collect();
    let mut copies: Vec<Copied> = Vec::new();
    let mut by_id: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    for l in live {
        let id = match l.id {
            Some(id) => id,
            None => {
                let id = new_id(&taken);
                replace_id(&join_rel(root, &l.rel), &id)?;
                taken.insert(id.clone());
                id
            }
        };
        by_id.entry(id).or_default().push(l.rel);
    }

    let mut moves: Vec<(Vec<String>, Vec<String>)> = Vec::new();
    let mut remembered: HashMap<String, String> = HashMap::new();
    for (id, mut paths) in by_id {
        if paths.len() > 1 {
            // Copies share an id; the path the db already knows keeps it.
            let keep = known.get(&id).and_then(|k| paths.iter().position(|p| p.join("/") == *k)).unwrap_or(0);
            let keeper = paths.remove(keep);
            if mode == Copies::Ignore {
                remembered.insert(id, keeper.join("/"));
                continue;
            }
            for copy in paths {
                let tail = tailed_id(&id, &taken);
                replace_id(&join_rel(root, &copy), &tail)?;
                taken.insert(tail.clone());
                remembered.insert(tail, copy.join("/"));
                copies.push(Copied { rel_parts: copy, original: keeper.clone() });
            }
            paths = vec![keeper];
        }
        let only = &paths[0];
        // can you find me?
        // A folder moved outside the app is found again by its id instead of showing as a ghost plus a new folder.
        if let Some(old) = known.get(&id).filter(|old| **old != only.join("/")) {
            let old_rel: Vec<String> = old.split('/').map(str::to_string).collect();
            if !still_listed(root, &old_rel) {
                moves.push((old_rel, only.clone()));
            }
        }
        remembered.insert(id, only.join("/"));
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
    for (id, path) in &remembered {
        if set_uid(data, path, id) {
            changed = true;
        }
    }
    copies.sort_by(|a, b| a.rel_parts.cmp(&b.rel_parts));
    Ok(Plan { changed, copies })
}

// The tail keeps the original id readable, and a copy of a copy counts on from the same original.
fn tailed_id(id: &str, taken: &HashSet<String>) -> String {
    let base = id.split('-').next().unwrap_or(id);
    let mut n = 2;
    loop {
        let tail = format!("{base}-{n}");
        if !taken.contains(&tail) {
            return tail;
        }
        n += 1;
    }
}

// A copy the user calls a new, independent project gets a fresh id without a tail.
pub fn detach(root: &Path, data: &mut Mapping, rel: &[String]) -> R<()> {
    let folder = join_rel(root, rel);
    let not_a_copy = || FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "the folder is not a marked copy".to_string() };
    let current = read_id(&folder).ok_or_else(not_a_copy)?;
    if !current.contains('-') {
        return Err(not_a_copy());
    }
    let Scan { live, known, .. } = scan(root, data)?;
    let taken: HashSet<String> = known.into_keys().chain(live.into_iter().filter_map(|l| l.id)).collect();
    let fresh = new_id(&taken);
    replace_id(&folder, &fresh)?;
    set_uid(data, &rel.join("/"), &fresh);
    Ok(())
}

// A copy made inside the app is tailed at once, by the same rule a refresh uses, so it never shares the original's id.
pub fn tail_copy(root: &Path, data: &mut Mapping, copy: &[String]) -> R<()> {
    let Scan { live, known, .. } = scan(root, data)?;
    let mut taken: HashSet<String> = known.into_keys().chain(live.into_iter().filter_map(|l| l.id)).collect();
    // The whole copy is walked, past the depth limit too, so no folder inside it keeps the original's id.
    let folder = join_rel(root, copy);
    let mut inside = vec![Live { rel: copy.to_vec(), id: read_id(&folder) }];
    walk(&mut copy.to_vec(), &folder, &mut inside, usize::MAX)?;
    for l in inside {
        let Some(id) = l.id else { continue };
        let tail = tailed_id(&id, &taken);
        replace_id(&join_rel(root, &l.rel), &tail)?;
        taken.insert(tail.clone());
        set_uid(data, &l.rel.join("/"), &tail);
    }
    Ok(())
}

fn set_uid(data: &mut Mapping, path: &str, uid: &str) -> bool {
    let rel: Vec<String> = path.split('/').map(str::to_string).collect();
    let Some(node) = get_node_mut(data, &rel) else { return false };
    if node.get("uid").and_then(Value::as_str) == Some(uid) {
        return false;
    }
    node.insert("uid".into(), uid.into());
    true
}

// Compares the real directory listing, because on Windows a case-only rename still resolves under the old name.
fn still_listed(root: &Path, rel: &[String]) -> bool {
    let Some((name, parent)) = rel.split_last() else { return false };
    let dir = join_rel(root, parent);
    dir.is_dir() && list_subdirs(&dir).is_ok_and(|names| names.contains(name))
}

fn walk(rel: &mut Vec<String>, dir: &Path, out: &mut Vec<Live>, limit: usize) -> R<()> {
    if rel.len() >= limit {
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
        walk(rel, &folder, out, limit)?;
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

// The flags stay, so tailing or detaching a copy keeps what the user set on it.
fn replace_id(folder: &Path, id: &str) -> R<()> {
    write_atomic(&folder.join(DATA_DIR).join(FOOTPRINT_FILENAME), footprint_text(id, &read_flags(folder)).as_bytes())
}

fn read_flags(folder: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(folder.join(DATA_DIR).join(FOOTPRINT_FILENAME)) else { return Vec::new() };
    let Ok(Value::Mapping(m)) = serde_yaml::from_str::<Value>(&text) else { return Vec::new() };
    m.get("flags").and_then(Value::as_sequence).map_or_else(Vec::new, |list| list.iter().filter_map(Value::as_str).map(str::to_string).collect())
}

// Keeps the folder's dragpath.json in step with its user flags; a folder without one, or with a broken one, is left as it is.
pub fn mirror_flags(folder: &Path, flags: &[String]) -> R<()> {
    let Some(id) = read_id(folder) else { return Ok(()) };
    write_atomic(&folder.join(DATA_DIR).join(FOOTPRINT_FILENAME), footprint_text(&id, flags).as_bytes())
}

fn footprint_text(id: &str, flags: &[String]) -> String {
    if flags.is_empty() {
        return format!("{{\"id\": \"{id}\"}}\n");
    }
    let list: Vec<String> = flags.iter().map(|f| format!("\"{f}\"")).collect();
    format!("{{\"id\": \"{id}\", \"flags\": [{}]}}\n", list.join(", "))
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
    use crate::test_support::TempDir;
    use crate::tree::tree_store::sync;

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
        assert!(!scan(dir.path(), &data).unwrap().is_clean());
        assert!(plan(dir.path(), &mut data, Copies::Tail).unwrap().changed);
        let first = read_id(&dir.path().join("100.A/PJ.001")).unwrap();
        assert_eq!(first.len(), ID_LEN);
        assert!(scan(dir.path(), &data).unwrap().is_clean());
        assert!(!plan(dir.path(), &mut data, Copies::Tail).unwrap().changed);
        assert_eq!(read_id(&dir.path().join("100.A/PJ.001")).unwrap(), first);
    }

    #[test]
    fn a_node_with_loose_files_gets_no_marker_until_it_is_acknowledged() {
        let dir = TempDir::new("footprint-waiting");
        dir.mkdir("100.A/01.Plain");
        fs::write(dir.path().join("100.A/01.Plain/loose.txt"), "l").unwrap();
        let mut data: Mapping = serde_yaml::from_str("100.A:\n  type: NODE-L1\n  01.Plain: {type: NODE-L2}\n").unwrap();
        plan(dir.path(), &mut data, Copies::Tail).unwrap();
        assert!(!dir.path().join("100.A/01.Plain/.ant").exists());
        assert!(scan(dir.path(), &data).unwrap().is_clean());
        fs::create_dir(dir.path().join("100.A/01.Plain/.ant")).unwrap();
        assert!(plan(dir.path(), &mut data, Copies::Tail).unwrap().changed);
        assert!(read_id(&dir.path().join("100.A/01.Plain")).is_some());
    }

    #[test]
    fn a_refresh_marks_data_nodes_too() {
        let dir = TempDir::new("footprint-marks");
        dir.mkdir("100.A/01.Plain");
        dir.mkdir("100.A/02.Data");
        let mut data: Mapping = serde_yaml::from_str("100.A:\n  type: NODE-L1\n  01.Plain: {type: NODE-L2}\n  02.Data: {type: NODE-L2, flags: data}\n").unwrap();
        assert!(plan(dir.path(), &mut data, Copies::Tail).unwrap().changed);
        assert!(read_id(&dir.path().join("100.A/01.Plain")).is_some());
        assert!(read_id(&dir.path().join("100.A/02.Data")).is_some());
        assert!(scan(dir.path(), &data).unwrap().is_clean());
    }

    #[test]
    fn a_folder_without_dot_ant_is_left_alone() {
        let dir = TempDir::new("footprint-plain");
        dir.mkdir("100.A/PJ.001");
        let mut data = Mapping::new();
        plan(dir.path(), &mut data, Copies::Tail).unwrap();
        assert!(!dir.path().join("100.A/PJ.001/.ant").exists());
    }

    #[test]
    fn a_folder_moved_outside_the_app_keeps_its_type_and_status() {
        let (dir, mut data) = storage();
        plan(dir.path(), &mut data, Copies::Tail).unwrap();
        fs::rename(dir.path().join("100.A/PJ.001"), dir.path().join("100.A/PJ.009")).unwrap();
        sync(dir.path(), &mut data).unwrap();
        assert!(plan(dir.path(), &mut data, Copies::Tail).unwrap().changed);
        let moved = get_node(&data, &rel(&["100.A", "PJ.009"])).unwrap();
        assert_eq!(moved.get("status").and_then(Value::as_str), Some("hold"));
        assert!(get_node(&data, &rel(&["100.A", "PJ.001"])).is_none());
        assert!(scan(dir.path(), &data).unwrap().is_clean());
    }

    fn copy_footprint(dir: &TempDir, from: &str, to: &str) {
        dir.mkdir(&format!("{to}/.ant"));
        fs::copy(dir.path().join(from).join(".ant/dragpath.json"), dir.path().join(to).join(".ant/dragpath.json")).unwrap();
    }

    fn uid(data: &Mapping, parts: &[&str]) -> Option<String> {
        get_node(data, &rel(parts)).and_then(|n| n.get("uid")).and_then(Value::as_str).map(str::to_string)
    }

    #[test]
    fn a_copied_folder_is_not_taken_for_a_move() {
        let (dir, mut data) = storage();
        plan(dir.path(), &mut data, Copies::Tail).unwrap();
        copy_footprint(&dir, "100.A/PJ.001", "100.A/PJ.002");
        sync(dir.path(), &mut data).unwrap();
        let result = plan(dir.path(), &mut data, Copies::Ignore).unwrap();
        assert!(!result.changed && result.copies.is_empty());
        assert!(get_node(&data, &rel(&["100.A", "PJ.001"])).is_some() && get_node(&data, &rel(&["100.A", "PJ.002"])).is_some());
        assert_eq!(read_id(&dir.path().join("100.A/PJ.002")), read_id(&dir.path().join("100.A/PJ.001")));
    }

    #[test]
    fn the_known_folder_keeps_the_id_and_each_copy_gets_a_tail_once() {
        let (dir, mut data) = storage();
        plan(dir.path(), &mut data, Copies::Tail).unwrap();
        let id = read_id(&dir.path().join("100.A/PJ.001")).unwrap();
        fs::write(dir.path().join("100.A/PJ.001/.ant/dragpath.json"), format!("{{\"id\": \"{id}\", \"flags\": [\"urgent\"]}}\n")).unwrap();
        copy_footprint(&dir, "100.A/PJ.001", "100.A/PJ.000");
        copy_footprint(&dir, "100.A/PJ.001", "100.A/PJ.002");
        sync(dir.path(), &mut data).unwrap();
        let result = plan(dir.path(), &mut data, Copies::Tail).unwrap();
        assert!(result.changed);
        let original = rel(&["100.A", "PJ.001"]);
        let expected = vec![
            Copied { rel_parts: rel(&["100.A", "PJ.000"]), original: original.clone() },
            Copied { rel_parts: rel(&["100.A", "PJ.002"]), original: original.clone() },
        ];
        assert_eq!(result.copies, expected);
        assert_eq!(read_id(&dir.path().join("100.A/PJ.001")), Some(id.clone()));
        let tails = [read_id(&dir.path().join("100.A/PJ.000")).unwrap(), read_id(&dir.path().join("100.A/PJ.002")).unwrap()];
        assert_eq!(tails.iter().filter(|t| t.starts_with(&format!("{id}-")) && t.len() == ID_LEN + 2).count(), 2);
        assert_ne!(tails[0], tails[1]);
        assert_eq!(uid(&data, &["100.A", "PJ.002"]), Some(tails[1].clone()));
        assert_eq!(read_flags(&dir.path().join("100.A/PJ.002")), ["urgent"]);
        assert!(scan(dir.path(), &data).unwrap().is_clean());
        let again = plan(dir.path(), &mut data, Copies::Tail).unwrap();
        assert!(!again.changed && again.copies.is_empty());
    }

    #[test]
    fn a_copy_of_a_copy_counts_on_from_the_original_id() {
        let taken: HashSet<String> = ["7K3M9Q2XH43O7Z4", "7K3M9Q2XH43O7Z4-2"].map(str::to_string).into();
        assert_eq!(tailed_id("7K3M9Q2XH43O7Z4", &taken), "7K3M9Q2XH43O7Z4-3");
        assert_eq!(tailed_id("7K3M9Q2XH43O7Z4-2", &taken), "7K3M9Q2XH43O7Z4-3");
    }

    #[test]
    fn a_copy_then_move_carries_the_entry_to_one_folder_and_tails_the_other() {
        let (dir, mut data) = storage();
        plan(dir.path(), &mut data, Copies::Tail).unwrap();
        copy_footprint(&dir, "100.A/PJ.001", "100.A/PJ.002");
        fs::rename(dir.path().join("100.A/PJ.001"), dir.path().join("100.A/PJ.009")).unwrap();
        sync(dir.path(), &mut data).unwrap();
        let result = plan(dir.path(), &mut data, Copies::Tail).unwrap();
        assert_eq!(result.copies.len(), 1);
        let keeper = result.copies[0].original.clone();
        assert_eq!(get_node(&data, &keeper).unwrap().get("status").and_then(Value::as_str), Some("hold"));
        assert!(get_node(&data, &rel(&["100.A", "PJ.001"])).is_none());
        assert!(read_id(&join_rel(dir.path(), &result.copies[0].rel_parts)).unwrap().contains('-'));
        assert!(scan(dir.path(), &data).unwrap().is_clean());
    }

    #[test]
    fn a_tailed_copy_is_never_taken_for_the_deleted_original() {
        let (dir, mut data) = storage();
        plan(dir.path(), &mut data, Copies::Tail).unwrap();
        copy_footprint(&dir, "100.A/PJ.001", "100.A/PJ.002");
        sync(dir.path(), &mut data).unwrap();
        plan(dir.path(), &mut data, Copies::Tail).unwrap();
        let tail = read_id(&dir.path().join("100.A/PJ.002")).unwrap();
        fs::remove_dir_all(dir.path().join("100.A/PJ.001")).unwrap();
        sync(dir.path(), &mut data).unwrap();
        let result = plan(dir.path(), &mut data, Copies::Tail).unwrap();
        assert!(!result.changed && result.copies.is_empty());
        assert_eq!(get_node(&data, &rel(&["100.A", "PJ.001"])).unwrap().get("status").and_then(Value::as_str), Some("hold"));
        assert_eq!(uid(&data, &["100.A", "PJ.002"]), Some(tail));
    }

    #[test]
    fn a_moved_tailed_copy_is_followed_by_its_own_id() {
        let (dir, mut data) = storage();
        plan(dir.path(), &mut data, Copies::Tail).unwrap();
        copy_footprint(&dir, "100.A/PJ.001", "100.A/PJ.002");
        sync(dir.path(), &mut data).unwrap();
        plan(dir.path(), &mut data, Copies::Tail).unwrap();
        get_node_mut(&mut data, &rel(&["100.A", "PJ.002"])).unwrap().insert("status".into(), "done".into());
        fs::rename(dir.path().join("100.A/PJ.002"), dir.path().join("100.A/PJ.008")).unwrap();
        sync(dir.path(), &mut data).unwrap();
        assert!(plan(dir.path(), &mut data, Copies::Tail).unwrap().changed);
        assert_eq!(get_node(&data, &rel(&["100.A", "PJ.008"])).unwrap().get("status").and_then(Value::as_str), Some("done"));
        assert_eq!(get_node(&data, &rel(&["100.A", "PJ.001"])).unwrap().get("status").and_then(Value::as_str), Some("hold"));
    }

    #[test]
    fn a_case_only_rename_migrates_the_entry() {
        let (dir, mut data) = storage();
        plan(dir.path(), &mut data, Copies::Tail).unwrap();
        fs::rename(dir.path().join("100.A/PJ.001"), dir.path().join("100.A/pj.001")).unwrap();
        sync(dir.path(), &mut data).unwrap();
        assert!(plan(dir.path(), &mut data, Copies::Tail).unwrap().changed);
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
        plan(dir.path(), &mut data, Copies::Tail).unwrap();
        fs::rename(dir.path().join("100.A"), dir.path().join("200.B")).unwrap();
        fs::rename(dir.path().join("200.B/PJ.001"), dir.path().join("200.B/PJ.002")).unwrap();
        sync(dir.path(), &mut data).unwrap();
        assert!(plan(dir.path(), &mut data, Copies::Tail).unwrap().changed);
        let child = get_node(&data, &rel(&["200.B", "PJ.002"])).unwrap();
        assert_eq!(child.get("status").and_then(Value::as_str), Some("hold"));
        assert!(get_node(&data, &rel(&["100.A"])).is_none());
    }

    #[test]
    fn plan_writes_the_uid_into_the_tree() {
        let (dir, mut data) = storage();
        assert!(plan(dir.path(), &mut data, Copies::Tail).unwrap().changed);
        let node = get_node(&data, &rel(&["100.A", "PJ.001"])).unwrap();
        assert_eq!(node.get("uid").and_then(Value::as_str), read_id(&dir.path().join("100.A/PJ.001")).as_deref());
    }
}
