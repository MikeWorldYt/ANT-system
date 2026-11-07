// Gives every folder that has a .ant folder a lasting id, so moves made outside the app are recognised.
#[cfg(test)]
mod tests;

use std::collections::hash_map::RandomState;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{BuildHasher, Hasher};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_yaml::{Mapping, Value};

use crate::domain::tree_types::{child_entries, flags_in, type_base, META_KEYS};
use crate::domain::config::{DATA_DIR, HIDDEN_FLAG};
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::{io_err, join_rel};
use crate::infra::scan::list_subdirs;
use crate::domain::model::{Copied, Copies};
use crate::infra::footprint_file::{read_id, replace_id};
use crate::infra::storage::db::hide;
use crate::services::tree::depth;
use crate::services::tree::tree_node::is_reserved;
use crate::services::tree::tree_store::{delete_key, get_node, get_node_mut};

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
    walk(Some(data), &mut Vec::new(), root, &mut live, depth::limit(root)?)?;
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
        if dir.is_dir() && !dir.join(DATA_DIR).exists() && crate::services::intake::waiting_files(&dir, &rel, node)? == 0 {
            out.push(path.clone());
        }
    }
    Ok(out)
}

// A folder the app cannot write to, such as an encrypted one, stays unmarked and must not hide the rest of the tree.
fn create_markers(root: &Path, paths: &[String]) -> Vec<String> {
    let mut warnings = Vec::new();
    for path in paths {
        let dir = join_rel(root, &rel_of(path)).join(DATA_DIR);
        match fs::create_dir(&dir) {
            Ok(()) => hide(&dir),
            Err(e) => warnings.push(io_err("create info folder", &dir, &e).to_string()),
        }
    }
    warnings
}

fn rel_of(path: &str) -> Vec<String> {
    path.split('/').map(str::to_string).collect()
}

pub struct Plan {
    pub changed: bool,
    pub copies: Vec<Copied>,
    pub warnings: Vec<String>,
}

// Writes missing ids to disk, carries moved folders' entries to their new place and stores each uid in the tree.
pub fn plan(root: &Path, data: &mut Mapping, mode: Copies) -> R<Plan> {
    let mut nodes = Vec::new();
    visit(data, &mut Vec::new(), &mut nodes);
    let warnings = create_markers(root, &unmarked(root, &nodes)?);
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
    Ok(Plan { changed, copies, warnings })
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
    walk(None, &mut copy.to_vec(), &folder, &mut inside, usize::MAX)?;
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

// Hidden folders are not read, so their ids and marks are left out too.
fn walk(data: Option<&Mapping>, rel: &mut Vec<String>, dir: &Path, out: &mut Vec<Live>, limit: usize) -> R<()> {
    if rel.len() >= limit {
        return Ok(());
    }
    for name in list_subdirs(dir)? {
        if META_KEYS.contains(&name.as_str()) || (rel.is_empty() && is_reserved(std::slice::from_ref(&name))) {
            continue;
        }
        let folder = dir.join(&name);
        rel.push(name);
        if data.and_then(|d| get_node(d, rel)).is_some_and(|node| flags_in(node).iter().any(|f| f == HIDDEN_FLAG)) {
            rel.pop();
            continue;
        }
        if folder.join(DATA_DIR).is_dir() {
            out.push(Live { rel: rel.clone(), id: read_id(&folder) });
        }
        walk(data, rel, &folder, out, limit)?;
        rel.pop();
    }
    Ok(())
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
