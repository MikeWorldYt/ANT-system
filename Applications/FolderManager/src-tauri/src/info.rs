// Reads and updates a folder's .ant/info.json, the project metadata that travels with the folder.
use std::fs;
use std::path::Path;

use serde::Serialize;
use serde_yaml::{Mapping, Value};

use crate::config::{DATA_DIR, INFO_FILENAME};
use crate::db::hide;
use crate::error::{FmError, R};
use crate::fs_safe::{io_err, write_atomic};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoField {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfo {
    pub fields: Vec<InfoField>,
    pub error: Option<String>,
}

impl NodeInfo {
    fn failed(error: String) -> Self {
        Self { fields: Vec::new(), error: Some(error) }
    }
}

// JSON is read as YAML flow syntax, which spares a dependency and still rejects broken JSON.
pub fn read_info(folder: &Path) -> Option<NodeInfo> {
    let path = folder.join(DATA_DIR).join(INFO_FILENAME);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => return Some(NodeInfo::failed(format!("{INFO_FILENAME} could not be read: {e}"))),
    };
    Some(match serde_yaml::from_str::<Value>(text.trim_start_matches('\u{feff}')) {
        Ok(Value::Mapping(entries)) => NodeInfo { fields: fields_of(&entries), error: None },
        Ok(_) => NodeInfo::failed(format!("{INFO_FILENAME} must hold an object")),
        Err(e) => NodeInfo::failed(format!("{INFO_FILENAME} is not valid JSON: {e}")),
    })
}

fn fields_of(entries: &Mapping) -> Vec<InfoField> {
    entries
        .iter()
        .filter_map(|(k, v)| Some(InfoField { key: k.as_str()?.to_string(), value: scalar_text(v)? }))
        .collect()
}

// Nulls and nested objects carry nothing a search or the details view can show.
fn scalar_text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Sequence(items) => {
            let parts: Vec<String> = items.iter().filter_map(scalar_text).collect();
            (!parts.is_empty()).then(|| parts.join(", "))
        }
        _ => None,
    }
}

// Keys that name the same person as `owner`; an existing one is updated instead of adding a second.
pub const OWNER_KEYS: [&str; 3] = ["owner", "holder", "client"];

// The first scalar stored under any of the keys, compared case-insensitively.
pub fn field_of<'a>(info: &'a NodeInfo, keys: &[&str]) -> Option<&'a str> {
    info.fields.iter().find(|f| keys.iter().any(|k| f.key.eq_ignore_ascii_case(k))).map(|f| f.value.as_str())
}

// Sets keys in the folder's info.json, keeping every other entry; a broken file is never overwritten.
pub fn write_info(folder: &Path, updates: &[(&[&str], &str, &str)]) -> R<()> {
    let dir = folder.join(DATA_DIR);
    let path = dir.join(INFO_FILENAME);
    let mut entries = match fs::read_to_string(&path) {
        Ok(text) => match serde_yaml::from_str::<Value>(text.trim_start_matches('\u{feff}')) {
            Ok(Value::Mapping(entries)) => entries,
            _ => return Err(FmError::Internal { message: format!("{INFO_FILENAME} is not valid; fix it before editing") }),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Mapping::new(),
        Err(e) => return Err(io_err("read info", &path, &e)),
    };
    for (aliases, default_key, value) in updates {
        let existing = entries
            .keys()
            .find_map(|k| k.as_str().filter(|k| aliases.iter().any(|a| a.eq_ignore_ascii_case(k))))
            .map(str::to_string);
        entries.insert(existing.unwrap_or_else(|| default_key.to_string()).into(), Value::String(value.to_string()));
    }
    if !dir.is_dir() {
        fs::create_dir(&dir).map_err(|e| io_err("create info folder", &dir, &e))?;
        hide(&dir);
    }
    write_atomic(&path, to_json(&Value::Mapping(entries)).as_bytes())
}

fn to_json(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => json_string(s),
        Value::Sequence(items) => format!("[{}]", items.iter().map(to_json).collect::<Vec<_>>().join(", ")),
        Value::Mapping(m) => {
            let body: Vec<String> =
                m.iter().map(|(k, v)| format!("  {}: {}", json_string(k.as_str().unwrap_or_default()), to_json(v))).collect();
            format!("{{\n{}\n}}\n", body.join(",\n"))
        }
        Value::Tagged(t) => to_json(&t.value),
    }
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree_store::test_support::TempDir;

    fn with_info(tag: &str, text: &str) -> TempDir {
        let dir = TempDir::new(tag);
        dir.mkdir(".ant");
        fs::write(dir.path().join(".ant/info.json"), text).unwrap();
        dir
    }

    #[test]
    fn a_folder_without_info_gives_none() {
        assert!(read_info(TempDir::new("info-none").path()).is_none());
    }

    #[test]
    fn valid_json_keeps_scalars_in_file_order() {
        let dir = with_info("info-ok", r#"{"holder": "Mike", "created": "26-10-11", "tag": null, "hours": 3, "tags": ["a", "b"]}"#);
        let info = read_info(dir.path()).unwrap();
        assert_eq!(info.error, None);
        let pairs: Vec<(&str, &str)> = info.fields.iter().map(|f| (f.key.as_str(), f.value.as_str())).collect();
        assert_eq!(pairs, [("holder", "Mike"), ("created", "26-10-11"), ("hours", "3"), ("tags", "a, b")]);
    }

    #[test]
    fn missing_commas_become_an_error_instead_of_fields() {
        let dir = with_info("info-bad", "{\n  \"holder\":\"Mike\",\n  \"created\": \"26-10-11\"\n  \"tag\": null\n}");
        let info = read_info(dir.path()).unwrap();
        assert!(info.fields.is_empty() && info.error.as_deref().is_some_and(|e| e.contains("not valid JSON")));
    }

    #[test]
    fn writing_keeps_other_entries_and_updates_an_alias_in_place() {
        let dir = with_info("info-write", r#"{"holder": "Mike", "hours": 3, "note": "say \"hi\""}"#);
        write_info(dir.path(), &[(&OWNER_KEYS, "owner", "Ana"), (&["status"], "status", "hold")]).unwrap();
        let info = read_info(dir.path()).unwrap();
        let pairs: Vec<(&str, &str)> = info.fields.iter().map(|f| (f.key.as_str(), f.value.as_str())).collect();
        assert_eq!(pairs, [("holder", "Ana"), ("hours", "3"), ("note", "say \"hi\""), ("status", "hold")]);
    }

    #[test]
    fn writing_creates_the_hidden_folder_and_refuses_a_broken_file() {
        let dir = TempDir::new("info-create");
        write_info(dir.path(), &[(&["status"], "status", "unknown")]).unwrap();
        assert_eq!(read_info(dir.path()).unwrap().fields.len(), 1);
        fs::write(dir.path().join(".ant/info.json"), "{oops").unwrap();
        assert!(write_info(dir.path(), &[(&["status"], "status", "hold")]).is_err());
    }

    #[test]
    fn a_non_object_is_an_error() {
        assert!(read_info(with_info("info-list", "[1, 2]").path()).unwrap().error.is_some());
    }
}
