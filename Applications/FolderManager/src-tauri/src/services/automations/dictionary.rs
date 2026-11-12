// Reads the `.ant/dictionary.md` files from the storage root down to a folder, so a node's dictionary reaches every folder inside it.
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};

use crate::domain::error::{FmError, R};
use crate::domain::names::editable_part;
use crate::infra::fs_safe::{io_err, join_rel, resolve, write_atomic, StorageLock};

pub const DICTIONARY_FILE: &str = "dictionary.md";
pub const NO_GROUP: &str = "do not group";
const NODE_PREFIX: &str = "NODE.";
const UP_PREFIX: &str = "NODE^.";

#[derive(Debug, Clone, Default)]
pub struct Entry {
    pub folder: String,
    pub terms: Vec<String>,
    pub topics: Vec<String>,
    pub only: Vec<String>,
    pub exclude: Vec<String>,
    pub exts: Vec<String>,
    pub patterns: Vec<Regex>,
    // A match on a "Do not group" entry keeps the file where it is, whatever else it names.
    pub group: bool,
    // A `NODE.` or `NODE^.` heading sends its files to that node instead of a subfolder.
    pub node: Option<Vec<String>>,
}

fn list(value: &str) -> impl Iterator<Item = String> + '_ {
    value.split(',').map(str::trim).filter(|t| !t.is_empty()).map(str::to_string)
}

fn exts(value: &str) -> impl Iterator<Item = String> + '_ {
    value.split([' ', ',']).filter(|e| !e.is_empty()).map(|e| format!(".{}", e.trim_start_matches('.').to_lowercase()))
}

// Obsidian's `%%` blocks are notes for the reader, so the app skips them.
fn without_comments(text: &str) -> String {
    text.split("%%").step_by(2).collect()
}

// One entry as written in the file, before its patterns compile; the MCP tools read and write dictionaries in this shape.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Spec {
    pub folder: String,
    pub term: Vec<String>,
    pub topics: Vec<String>,
    pub only: Vec<String>,
    pub exclude: Vec<String>,
    pub ext: Vec<String>,
    pub pattern: Vec<String>,
}

// The first `#` heading is the title; every other heading opens an entry named as its folder, refined by `- key: values` lines.
pub fn parse(text: &str) -> Vec<Spec> {
    let mut specs: Vec<Spec> = Vec::new();
    let mut titled = false;
    for line in without_comments(text).lines().map(str::trim) {
        if let Some(heading) = line.strip_prefix('#') {
            let name = heading.trim_start_matches('#').trim();
            if !titled && !heading.starts_with('#') {
                titled = true;
            } else if !name.is_empty() {
                specs.push(Spec { folder: name.to_string(), ..Spec::default() });
            }
            continue;
        }
        let (Some(option), Some(spec)) = (line.strip_prefix("- "), specs.last_mut()) else { continue };
        let Some((key, value)) = option.split_once(':') else { continue };
        let value = value.trim();
        match key.trim() {
            "term" => spec.term.extend(list(value)),
            "topics" => spec.topics.extend(list(value)),
            "only" => spec.only.extend(list(value)),
            "exclude" => spec.exclude.extend(list(value)),
            "ext" => spec.ext.extend(exts(value)),
            "pattern" if !value.is_empty() => spec.pattern.push(value.to_string()),
            _ => {}
        }
    }
    specs
}

pub fn compile(spec: &Spec, source: &Path) -> R<Entry> {
    let patterns = spec
        .pattern
        .iter()
        .map(|p| RegexBuilder::new(p).case_insensitive(true).build().map_err(|e| FmError::Internal { message: format!("{}: pattern {p}: {e}", source.display()) }))
        .collect::<R<Vec<Regex>>>()?;
    let ext = spec.ext.iter().flat_map(|e| exts(e)).collect();
    Ok(Entry {
        folder: spec.folder.clone(),
        terms: spec.term.clone(),
        topics: spec.topics.clone(),
        only: spec.only.clone(),
        exclude: spec.exclude.clone(),
        exts: ext,
        patterns,
        group: spec.folder.to_lowercase() != NO_GROUP,
        node: None,
    })
}

const HEADER: &str = "%%\nSee the documentation to build the entries correctly:\nhttps://ant.mikeworld.dev/en/advanced/automations#dictionaries/\n%%\n";

// Always the same layout, so a dictionary an agent writes reads like one a person wrote.
pub fn render(title: &str, specs: &[Spec]) -> String {
    let mut out = format!("{HEADER}\n# Dictionary for {title}\n");
    for spec in specs {
        out.push_str(&format!("\n## {}\n", spec.folder.trim()));
        for (key, values, joint) in [("term", &spec.term, ", "), ("topics", &spec.topics, ", "), ("only", &spec.only, ", "), ("exclude", &spec.exclude, ", "), ("ext", &spec.ext, " ")] {
            if !values.is_empty() {
                out.push_str(&format!("- {key}: {}\n", values.join(joint)));
            }
        }
        for pattern in &spec.pattern {
            out.push_str(&format!("- pattern: {pattern}\n"));
        }
    }
    out
}

pub fn file_of(root: &Path, folder: &[String]) -> std::path::PathBuf {
    join_rel(root, folder).join(".ant").join(DICTIONARY_FILE)
}

// Callers outside the app (the MCP) pass any path, so it must name a folder inside the storage.
fn checked_folder(root: &Path, folder: &[String]) -> R<()> {
    if resolve(root, folder)?.is_dir() {
        return Ok(());
    }
    Err(FmError::InvalidPath { rel_parts: folder.to_vec(), reason: "not a folder of the storage".to_string() })
}

pub fn read_specs(root: &Path, folder: &[String]) -> R<Option<Vec<Spec>>> {
    checked_folder(root, folder)?;
    let path = file_of(root, folder);
    if !path.is_file() {
        return Ok(None);
    }
    fs::read_to_string(&path).map(|text| Some(parse(&text))).map_err(|e| io_err("read the dictionary", &path, &e))
}

// A spec that would not read back the same, or whose pattern does not compile, is refused before anything is written.
pub fn write(root: &Path, folder: &[String], specs: &[Spec]) -> R<std::path::PathBuf> {
    let _lock = StorageLock::acquire(root)?;
    checked_folder(root, folder)?;
    let path = file_of(root, folder);
    let mut names = HashSet::new();
    for spec in specs {
        let name = spec.folder.trim();
        let bad = |what: &str| FmError::InvalidName { name: name.to_string(), reason: what.to_string() };
        if name.is_empty() || name.contains(['#', '%', '\n']) {
            return Err(bad("an entry needs a folder name without #, % or line breaks"));
        }
        if !names.insert(name.to_lowercase()) {
            return Err(bad("two entries share this folder name"));
        }
        if spec.term.iter().chain(&spec.topics).chain(&spec.only).chain(&spec.exclude).any(|t| t.contains([',', '\n', '%'])) {
            return Err(bad("a term cannot hold a comma, % or a line break"));
        }
        if spec.term.is_empty() && spec.topics.is_empty() && spec.only.is_empty() && spec.ext.is_empty() && spec.pattern.is_empty() {
            return Err(bad("an entry needs at least one term, topic, only, ext or pattern"));
        }
        compile(spec, &path)?;
    }
    let title = folder.last().map_or("the storage", String::as_str);
    let dir = join_rel(root, folder).join(".ant");
    fs::create_dir_all(&dir).map_err(|e| io_err("create the .ant folder", &dir, &e))?;
    write_atomic(&path, render(title, specs).as_bytes())?;
    Ok(path)
}

fn subdirs(dir: &Path) -> R<Vec<String>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| io_err("list folder", dir, &e))? {
        let entry = entry.map_err(|e| io_err("list folder", dir, &e))?;
        if entry.path().is_dir() {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    names.sort();
    Ok(names)
}

// Spaces, dashes and underscores read alike, so "Blue Archive" names "07.Blue-Archive".
fn same_node(a: &str, b: &str) -> bool {
    let key = |s: &str| s.trim().to_lowercase().replace([' ', '_'], "-");
    key(a) == key(b)
}

// `NODE.` names a sibling of the review folder; `NODE^.` climbs one level and names a sibling of the dictionary's folder.
fn node_name(heading: &str) -> Option<(bool, &str)> {
    [(false, NODE_PREFIX), (true, UP_PREFIX)].into_iter().find_map(|(up, prefix)| {
        heading.get(..prefix.len()).filter(|p| p.eq_ignore_ascii_case(prefix)).map(|_| (up, heading[prefix.len()..].trim()))
    })
}

// Found by name, not id, so reordering never breaks the heading; the drop then decides between the node itself and its review folder.
fn find_node(root: &Path, at: &[String], up: bool, name: &str) -> R<Option<Vec<String>>> {
    if up && at.is_empty() {
        return Ok(None);
    }
    let mut rel = if up { at[..at.len() - 1].to_vec() } else { at.to_vec() };
    let start = rel.len();
    // "Blue-Archive/Fanarts" walks down one folder per segment, each found by name too.
    for segment in name.split('/').map(str::trim).filter(|s| !s.is_empty()) {
        let Some(node) = subdirs(&join_rel(root, &rel))?.into_iter().find(|n| same_node(editable_part(n), segment)) else { return Ok(None) };
        rel.push(node);
    }
    Ok((rel.len() > start).then_some(rel))
}

// A renamed node keeps the headings that reach it: `NODE.` in its parent's dictionary, `NODE^.` in its siblings'.
pub fn rename_node_refs(root: &Path, parent: &[String], old: &str, new: &str) -> Vec<String> {
    let base = join_rel(root, parent);
    let siblings = match subdirs(&base) {
        Ok(names) => names.into_iter().map(|n| (true, base.join(n))),
        Err(e) => return vec![format!("Dictionaries not updated: {e}")],
    };
    let mut warnings = Vec::new();
    for (up, path) in siblings.chain(std::iter::once((false, base.clone()))).map(|(up, d)| (up, d.join(".ant").join(DICTIONARY_FILE))).filter(|(_, p)| p.is_file()) {
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) => {
                warnings.push(format!("Dictionary not updated: {}", io_err("read the dictionary", &path, &e)));
                continue;
            }
        };
        let prefix = if up { UP_PREFIX } else { NODE_PREFIX };
        let mut changed = false;
        let lines: Vec<String> = text
            .split_inclusive('\n')
            .map(|line| {
                let hashes = line.len() - line.trim_start_matches('#').len();
                match node_name(line[hashes..].trim()) {
                    Some((u, name)) if hashes > 0 && u == up && same_node(name.split('/').next().unwrap_or(name), old) => {
                        changed = true;
                        let end = &line[line.trim_end_matches(['\r', '\n']).len()..];
                        let tail = name.find('/').map_or("", |i| &name[i..]);
                        format!("{} {prefix}{new}{tail}{end}", &line[..hashes])
                    }
                    _ => line.to_string(),
                }
            })
            .collect();
        if changed {
            if let Err(e) = write_atomic(&path, lines.concat().as_bytes()) {
                warnings.push(format!("Dictionary not updated: {e}"));
            }
        }
    }
    warnings
}

// A closer dictionary adds to the farther ones and replaces their entries of the same name.
pub fn load(root: &Path, folder: &[String]) -> R<Vec<Entry>> {
    let mut entries: Vec<Entry> = Vec::new();
    let mut at: HashMap<String, usize> = HashMap::new();
    for end in 0..=folder.len() {
        let path = join_rel(root, &folder[..end]).join(".ant").join(DICTIONARY_FILE);
        if !path.is_file() {
            continue;
        }
        let text = fs::read_to_string(&path).map_err(|e| io_err("read the dictionary", &path, &e))?;
        for mut entry in parse(&text).iter().map(|spec| compile(spec, &path)).collect::<R<Vec<_>>>()? {
            if let Some((up, name)) = node_name(&entry.folder) {
                entry.node = find_node(root, &folder[..end], up, name)?;
                entry.group = entry.node.is_some();
            }
            match at.get(&entry.folder.to_lowercase()) {
                Some(&i) => entries[i] = entry,
                None => {
                    at.insert(entry.folder.to_lowercase(), entries.len());
                    entries.push(entry);
                }
            }
        }
    }
    Ok(entries)
}
