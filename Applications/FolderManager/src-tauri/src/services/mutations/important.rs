// The important flag pins a workboard to the top of its board column; only workboards take it.
use std::path::Path;

use serde_yaml::Value;

use super::{editable_target, finish, load_synced_at, parent_of, register_ids, Mutation};
use crate::domain::tree_types::type_base;
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::StorageLock;
use crate::infra::storage::info::write_info;
use crate::services::tree::tree_store;

pub fn set_important(root: &Path, rel: &[String], on: bool) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced_at(root, parent_of(rel))?;
    let target = editable_target(root, &data, rel, "mark as important")?;
    if type_base(&target.node_type) != "WKB" {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "only workboards can be marked as important".to_string() });
    }
    write_info(&target.abs, &[(&["important"], "important", on.then_some(Value::Bool(true)))])?;
    let mut warnings = Vec::new();
    let moved = register_ids(root, &mut data, &mut warnings);
    tree_store::save(root, &data)?;
    Ok(finish(root, &data, synced || moved, Some(rel.to_vec()), warnings, vec![rel.to_vec()]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::EditableType;
    use crate::services::mutations::set_node_type;
    use crate::test_support::*;

    #[test]
    fn important_is_a_flag_in_info_json_that_can_be_removed_again() {
        let dir = academic("important");
        let p = rel(&["100.ACADEMIC", "02.Bio"]);
        set_node_type(dir.path(), &p, EditableType::Wkb, None).unwrap();
        set_important(dir.path(), &p, true).unwrap();
        let file = dir.path().join("100.ACADEMIC/02.Bio/.ant/info.json");
        assert!(std::fs::read_to_string(&file).unwrap().contains("\"important\": true"));
        set_important(dir.path(), &p, false).unwrap();
        assert!(!std::fs::read_to_string(&file).unwrap().contains("important"));
        assert!(matches!(set_important(dir.path(), &rel(&["100.ACADEMIC", "03.Ghost"]), true), Err(FmError::GhostNotAllowed { .. })));
    }

    #[test]
    fn only_workboards_take_the_flag() {
        let dir = academic("important-wkb-only");
        let node = rel(&["100.ACADEMIC", "02.Bio"]);
        assert!(matches!(set_important(dir.path(), &node, true), Err(FmError::InvalidPath { .. })));
        assert!(matches!(set_important(dir.path(), &rel(&["100.ACADEMIC"]), true), Err(FmError::InvalidPath { .. }) | Err(FmError::Reserved { .. })));
        assert!(!dir.path().join("100.ACADEMIC/02.Bio/.ant/info.json").exists());
    }
}
