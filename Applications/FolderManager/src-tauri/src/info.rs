// Reads a folder's .ant/info.json, the project metadata that travels with the folder.
use std::fs;
use std::path::Path;

use serde::Serialize;
use serde_yaml::{Mapping, Value};

use crate::config::{DATA_DIR, INFO_FILENAME};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::yaml_store::test_support::TempDir;

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
    fn a_non_object_is_an_error() {
        assert!(read_info(with_info("info-list", "[1, 2]").path()).unwrap().error.is_some());
    }
}
