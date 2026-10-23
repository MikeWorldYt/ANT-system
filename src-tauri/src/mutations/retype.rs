// The type lives in the tree entry, while a workboard's status and owner travel in its own info.json.
use std::path::Path;

use serde_yaml::Value;

use super::{finish, guard_frozen, load_synced, locate, Mutation};
use crate::ant_core::tree_types::{type_base, type_level};
use crate::config::{EditableType, WkbStatus};
use crate::error::{FmError, R};
use crate::fs_safe::StorageLock;
use crate::storage::info::{field_of, read_info, write_info, OWNER_KEYS};
use crate::tree::tree_store::{self, get_node, get_node_mut};

pub fn set_node_type(
    root: &Path,
    rel: &[String],
    base: EditableType,
    status: Option<WkbStatus>,
) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced(root)?;
    let target = locate(root, &data, rel)?;
    guard_frozen(rel, &target.node_type)?;
    let base_name = base.as_str();
    let new_type = match type_level(&target.node_type) {
        Some(level) => format!("{base_name}-{level}"),
        None => base_name.to_string(),
    };
    let node = get_node_mut(&mut data, rel)
        .ok_or_else(|| FmError::NotFound { rel_parts: rel.to_vec() })?;
    node.insert("type".into(), new_type.into());
    match (base, status) {
        (EditableType::Wkb, Some(s)) => {
            node.insert("status".into(), s.as_str().into());
        }
        (EditableType::Wkb, None) => {}
        _ => {
            node.remove("status");
        }
    }
    tree_store::save(root, &data)?;
    Ok(finish(root, &data, synced, Some(rel.to_vec()), Vec::new()))
}

// Status and owner live in the folder's info.json; a value that is missing there is written as "unknown".
pub fn set_details(root: &Path, rel: &[String], status: Option<WkbStatus>, owner: Option<&str>) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (data, synced) = load_synced(root)?;
    let target = locate(root, &data, rel)?;
    guard_frozen(rel, &target.node_type)?;
    if target.ghost {
        return Err(FmError::GhostNotAllowed { op: "edit".to_string() });
    }
    if type_base(&target.node_type) != "WKB" {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "only workboards have status and owner".to_string() });
    }
    let current = read_info(&target.abs);
    let kept = |keys: &[&str]| current.as_ref().and_then(|i| field_of(i, keys)).map(str::to_string);
    let yaml_status = get_node(&data, rel).and_then(|n| n.get("status")).and_then(Value::as_str).map(str::to_string);
    let status = status
        .map(|s| s.as_str().to_string())
        .or_else(|| kept(&["status"]))
        .or(yaml_status)
        .unwrap_or_else(|| "unknown".to_string());
    let owner = match owner.map(str::trim) {
        Some(o) if !o.is_empty() => o.chars().filter(|c| !c.is_control()).take(100).collect(),
        _ => kept(&OWNER_KEYS).unwrap_or_else(|| "unknown".to_string()),
    };
    write_info(&target.abs, &[(&["status"], "status", Some(status.into())), (&OWNER_KEYS, "owner", Some(owner.into()))])?;
    tree_store::save(root, &data)?;
    Ok(finish(root, &data, synced, Some(rel.to_vec()), Vec::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

    #[test]
    fn details_go_to_info_json_and_default_to_the_yaml_status_then_unknown() {
        let dir = academic("details");
        let p = rel(&["100.ACADEMIC", "02.Bio"]);
        assert!(set_details(dir.path(), &p, None, Some("Ana")).is_err(), "not a workboard yet");
        set_node_type(dir.path(), &p, EditableType::Wkb, Some(WkbStatus::Hold)).unwrap();
        let m = set_details(dir.path(), &p, None, Some(" Ana ")).unwrap();
        let info = std::fs::read_to_string(dir.path().join("100.ACADEMIC/02.Bio/.ant/info.json")).unwrap();
        assert!(info.contains("\"status\": \"hold\"") && info.contains("\"owner\": \"Ana\""), "{info}");
        assert_eq!(m.select, Some(p.clone()));
        set_details(dir.path(), &p, Some(WkbStatus::Done), Some("  ")).unwrap();
        let info = std::fs::read_to_string(dir.path().join("100.ACADEMIC/02.Bio/.ant/info.json")).unwrap();
        assert!(info.contains("\"status\": \"done\"") && info.contains("\"owner\": \"Ana\""), "{info}");
    }

    #[test]
    fn set_node_type_keeps_level_and_manages_status() {
        let dir = academic("retype");
        let p = rel(&["100.ACADEMIC", "02.Bio"]);
        set_node_type(dir.path(), &p, EditableType::Wkb, Some(WkbStatus::ForReview)).unwrap();
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "02.Bio"]), "WKB-L2");
        let status = |d: &Path| {
            get_node(&yaml(d), &p).unwrap().get("status").and_then(Value::as_str).map(str::to_string)
        };
        assert_eq!(status(dir.path()).as_deref(), Some("for-review"));
        set_node_type(dir.path(), &p, EditableType::Wkb, None).unwrap();
        assert_eq!(status(dir.path()).as_deref(), Some("for-review"));
        set_node_type(dir.path(), &p, EditableType::Portf, Some(WkbStatus::Done)).unwrap();
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "02.Bio"]), "PORTF-L2");
        assert_eq!(status(dir.path()), None);
    }

    #[test]
    fn set_node_type_allows_ghosts_but_not_locked_or_reserved() {
        let dir = academic("retype-refuse");
        let ghost = rel(&["100.ACADEMIC", "03.Ghost"]);
        set_node_type(dir.path(), &ghost, EditableType::Portf, None).unwrap();
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "03.Ghost"]), "PORTF-L2");
        let locked = set_node_type(dir.path(), &rel(&["100.ACADEMIC", "01.Math", "90.Math"]), EditableType::Node, None);
        assert!(matches!(locked, Err(FmError::Locked { .. })));
        let reserved = set_node_type(dir.path(), &rel(&["999.Settings"]), EditableType::Node, None);
        assert!(matches!(reserved, Err(FmError::Reserved { .. })));
    }
}
