// Favorites skip the frozen-type guard: every folder but the reserved roots can be starred.
use std::path::Path;

use serde_yaml::Value;

use super::{finish, load_synced, locate, register_ids, Mutation};
use crate::error::{FmError, R};
use crate::fs_safe::StorageLock;
use crate::storage::info::write_info;
use crate::tree::tree_node::is_reserved;
use crate::tree::tree_store;

// A favorite is a flag in the folder's own info.json, so it travels with the folder and is searchable.
pub fn set_favorite(root: &Path, rel: &[String], on: bool) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced(root)?;
    let target = locate(root, &data, rel)?;
    if is_reserved(rel) {
        return Err(FmError::Reserved { rel_parts: rel.to_vec() });
    }
    if target.ghost {
        return Err(FmError::GhostNotAllowed { op: "favorite".to_string() });
    }
    write_info(&target.abs, &[(&["favorite"], "favorite", on.then_some(Value::Bool(true)))])?;
    let mut warnings = Vec::new();
    register_ids(root, &mut data, &mut warnings);
    tree_store::save(root, &data)?;
    Ok(finish(root, &data, synced, Some(rel.to_vec()), warnings))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

    #[test]
    fn favorite_is_a_flag_in_info_json_that_can_be_removed_again() {
        let dir = academic("favorite");
        let p = rel(&["100.ACADEMIC", "02.Bio"]);
        set_favorite(dir.path(), &p, true).unwrap();
        let file = dir.path().join("100.ACADEMIC/02.Bio/.ant/info.json");
        assert!(std::fs::read_to_string(&file).unwrap().contains("\"favorite\": true"));
        set_favorite(dir.path(), &p, false).unwrap();
        assert!(!std::fs::read_to_string(&file).unwrap().contains("favorite"));
        assert!(matches!(set_favorite(dir.path(), &rel(&["100.ACADEMIC", "03.Ghost"]), true), Err(FmError::GhostNotAllowed { .. })));
    }
}
