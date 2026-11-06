// Brings level 2 and deeper folder names to their id form, logging each rename; names without an id are left for the user.
#[cfg(test)]
mod tests;

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use serde_yaml::{Mapping, Value};

use crate::domain::names::{has_node_id, is_workboard_name, sanitize_train_case};
use crate::domain::tree_types::type_base;
use crate::services::backup::stamp::local_date;
use crate::services::debug_log::logs_dir;
use crate::domain::config::OWNER_KEYS;
use crate::domain::error::R;
use crate::infra::fs_safe::{io_err, Journal, StorageLock};
use crate::infra::storage::info::{field_of, read_info, write_info};
use crate::services::tree::depth;
use crate::services::tree::tree_node::is_reserved;
use crate::services::tree::tree_store;

struct Rename {
    parent: String,
    old: String,
    new: String,
}

// What a workboard name carried beyond its tracker and id, in the legacy "(created) tag [owner]_name" form.
#[derive(Default)]
struct Meta {
    created: Option<String>,
    tag: Option<String>,
    owner: Option<String>,
}

pub fn tidy(root: &Path) -> R<()> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, _) = tree_store::load_synced(root)?;
    let limit = depth::limit(root)?;
    let mut journal = Journal::new();
    let mut renames = Vec::new();
    if let Err(e) = walk(root, &mut data, &mut Vec::new(), limit, &mut journal, &mut renames) {
        return Err(journal.abort("rename folders", e));
    }
    if renames.is_empty() {
        return Ok(());
    }
    if let Err(e) = tree_store::save(root, &data) {
        return Err(journal.abort("rename folders", e));
    }
    journal.commit();
    log(root, &renames)
}

// Only nodes and workboards are walked: app folders and the contents of a workboard keep their own names.
fn walk(dir: &Path, node: &mut Mapping, rel: &mut Vec<String>, limit: usize, journal: &mut Journal, renames: &mut Vec<Rename>) -> R<()> {
    let names: Vec<String> = node.iter().filter(|(_, v)| v.is_mapping()).filter_map(|(k, _)| k.as_str().map(str::to_string)).collect();
    for name in names {
        // A tree entry whose folder is gone has nothing to rename or record; the sync reports it as a ghost.
        if !dir.join(&name).is_dir() || (rel.is_empty() && is_reserved(std::slice::from_ref(&name))) {
            continue;
        }
        let level = rel.len() + 1;
        if level >= 2 && is_workboard_name(&name) {
            tidy_workboard(dir, node, rel, &name, journal, renames)?;
            continue;
        }
        let is_node = node.get(name.as_str()).and_then(|c| c.get("type")).and_then(Value::as_str).is_some_and(|t| type_base(t) == "NODE");
        if !is_node {
            continue;
        }
        let mut current = name.clone();
        if level >= 2 {
            if let Some(new) = trimmed(&name) {
                if is_free(dir, node, &name, &new) {
                    rename_entry(dir, node, rel, &name, &new, journal, renames)?;
                    current = new;
                }
            }
        }
        if level < limit {
            if let Some(child) = node.get_mut(current.as_str()).and_then(Value::as_mapping_mut) {
                rel.push(current.clone());
                walk(&dir.join(&current), child, rel, limit, journal, renames)?;
                rel.pop();
            }
        }
    }
    Ok(())
}

fn tidy_workboard(dir: &Path, node: &mut Mapping, rel: &[String], name: &str, journal: &mut Journal, renames: &mut Vec<Rename>) -> R<()> {
    let Some((new, meta)) = workboard(name) else { return Ok(()) };
    let mut current = name.to_string();
    if new != name && is_free(dir, node, name, &new) {
        rename_entry(dir, node, rel, name, &new, journal, renames)?;
        current = new;
    }
    record(&dir.join(&current), &meta)
}

// A case-only change is free: the old name is the same folder on a case-insensitive disk.
fn is_free(dir: &Path, node: &Mapping, from: &str, to: &str) -> bool {
    to.eq_ignore_ascii_case(from) || (!node.contains_key(to) && !dir.join(to).exists())
}

fn rename_entry(dir: &Path, node: &mut Mapping, rel: &[String], old: &str, new: &str, journal: &mut Journal, renames: &mut Vec<Rename>) -> R<()> {
    journal.rename(&dir.join(old), &dir.join(new))?;
    if let Some(entry) = node.remove(old) {
        node.insert(Value::String(new.to_string()), entry);
    }
    renames.push(Rename { parent: rel.join("/"), old: old.to_string(), new: new.to_string() });
    Ok(())
}

// "10.Math" stays, "Unit01.Math" becomes "01.Math": letters in front of a two-digit id are noise.
fn trimmed(name: &str) -> Option<String> {
    if has_node_id(name) {
        return None;
    }
    let at = name.find(|c: char| !c.is_alphabetic())?;
    let rest = &name[at..];
    (at > 0 && has_node_id(rest)).then(|| rest.to_string())
}

// "PJ.111_(23-05MAY-04) Large [c011]_presentation" becomes "PJ.111_Presentation" with its metadata; None leaves the name for the user.
fn workboard(name: &str) -> Option<(String, Meta)> {
    let (tracker, rest) = name.split_once('.')?;
    let id = rest.get(..3).filter(|id| id.bytes().all(|b| b.is_ascii_digit()))?;
    let tail = &rest[3..];
    let mut s = if tail.is_empty() { tail } else { tail.strip_prefix('_')? };
    let mut meta = Meta::default();
    if let Some(after) = s.strip_prefix('(') {
        let (created, tail) = after.split_once(')')?;
        meta.created = Some(short_date(created));
        s = tail;
    }
    if let Some((tag, after)) = s.split_once('[') {
        let (owner, tail) = after.split_once(']')?;
        meta.tag = non_empty(tag);
        meta.owner = non_empty(owner);
        s = tail;
    }
    let text = s.trim_matches(|c: char| c == '_' || c.is_whitespace()).replace('_', "-");
    let title = sanitize_train_case(&text);
    let canonical = if title.is_empty() { format!("{tracker}.{id}") } else { format!("{tracker}.{id}_{title}") };
    Some((canonical, meta))
}

// "23-05MAY-04" and "23-05-04" both become "23-05-04".
fn short_date(raw: &str) -> String {
    match raw.split('-').collect::<Vec<_>>().as_slice() {
        [year, month, day] => {
            let digits: String = month.chars().take_while(char::is_ascii_digit).collect();
            if digits.is_empty() { raw.to_string() } else { format!("{year}-{digits}-{day}") }
        }
        _ => raw.to_string(),
    }
}

fn non_empty(text: &str) -> Option<String> {
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

// A broken info.json is left as it is, so the user can fix it before anything is written over it.
fn record(folder: &Path, meta: &Meta) -> R<()> {
    let info = read_info(folder);
    if info.as_ref().is_some_and(|i| i.error.is_some()) {
        return Ok(());
    }
    let mut updates: Vec<(&[&str], &str, Option<Value>)> = Vec::new();
    if let Some(owner) = &meta.owner {
        updates.push((&OWNER_KEYS, "owner", Some(owner.as_str().into())));
    }
    if let Some(created) = &meta.created {
        updates.push((&["created"], "created", Some(created.as_str().into())));
    }
    if let Some(tag) = &meta.tag {
        updates.push((&["tag"], "tag", Some(tag.as_str().into())));
    }
    if !info.as_ref().is_some_and(|i| field_of(i, &["status"]).is_some()) {
        updates.push((&["status"], "status", Some("in-progress".into())));
    }
    if updates.is_empty() {
        return Ok(());
    }
    write_info(folder, &updates)
}

fn log(root: &Path, renames: &[Rename]) -> R<()> {
    let dir = logs_dir(root);
    fs::create_dir_all(&dir).map_err(|e| io_err("create log folder", &dir, &e))?;
    let path = dir.join(format!("{}_ANT-fm-renamed.log", local_date()));
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| io_err("write rename log", &path, &e))?;
    let text: String = renames.iter().map(|r| format!("{} : \"{}\" → \"{}\"\n", r.parent, r.old, r.new)).collect();
    file.write_all(text.as_bytes()).map_err(|e| io_err("write rename log", &path, &e))
}
