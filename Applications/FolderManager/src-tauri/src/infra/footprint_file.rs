// Reads and writes a folder's .ant/dragpath.json: its lasting id, its user flags and its hidden mark.
use std::fs;
use std::path::{Path, PathBuf};

use serde_yaml::{Mapping, Value};

use crate::domain::config::{DATA_DIR, FOOTPRINT_FILENAME};
use crate::domain::error::R;
use crate::infra::fs_safe::write_atomic;

fn footprint_path(folder: &Path) -> PathBuf {
    folder.join(DATA_DIR).join(FOOTPRINT_FILENAME)
}

// A missing, unreadable or non-object file reads as no footprint at all.
fn footprint(folder: &Path) -> Option<Mapping> {
    let text = fs::read_to_string(footprint_path(folder)).ok()?;
    match serde_yaml::from_str::<Value>(&text).ok()? {
        Value::Mapping(m) => Some(m),
        _ => None,
    }
}

// An unreadable or malformed file counts as no id, so a new one replaces it.
pub(crate) fn read_id(folder: &Path) -> Option<String> {
    footprint(folder)?.get("id").and_then(Value::as_str).filter(|id| !id.is_empty() && !id.contains(char::is_whitespace)).map(str::to_string)
}

// The flags stay, so tailing or detaching a copy keeps what the user set on it.
pub(crate) fn replace_id(folder: &Path, id: &str) -> R<()> {
    write_footprint(folder, id, &read_flags(folder), read_hidden(folder))
}

// Keeps the folder's dragpath.json in step with its user flags; a folder without one, or with a broken one, is left as it is.
pub fn mirror_flags(folder: &Path, flags: &[String]) -> R<()> {
    let Some(id) = read_id(folder) else { return Ok(()) };
    write_footprint(folder, &id, flags, read_hidden(folder))
}

// Writes or drops the "hidden" key; a folder that is not hidden carries no key at all.
pub fn mirror_hidden(folder: &Path, hidden: bool) -> R<()> {
    let Some(id) = read_id(folder) else { return Ok(()) };
    write_footprint(folder, &id, &read_flags(folder), hidden)
}

pub(crate) fn read_flags(folder: &Path) -> Vec<String> {
    let Some(m) = footprint(folder) else { return Vec::new() };
    m.get("flags").and_then(Value::as_sequence).map_or_else(Vec::new, |list| list.iter().filter_map(Value::as_str).map(str::to_string).collect())
}

pub fn read_hidden(folder: &Path) -> bool {
    footprint(folder).is_some_and(|m| m.get("hidden") == Some(&Value::Bool(true)))
}

fn write_footprint(folder: &Path, id: &str, flags: &[String], hidden: bool) -> R<()> {
    write_atomic(&footprint_path(folder), footprint_text(id, flags, hidden).as_bytes())
}

fn footprint_text(id: &str, flags: &[String], hidden: bool) -> String {
    let mut parts = vec![format!("\"id\": \"{id}\"")];
    if !flags.is_empty() {
        let list: Vec<String> = flags.iter().map(|f| format!("\"{f}\"")).collect();
        parts.push(format!("\"flags\": [{}]", list.join(", ")));
    }
    if hidden {
        parts.push("\"hidden\": true".to_string());
    }
    format!("{{{}}}\n", parts.join(", "))
}
