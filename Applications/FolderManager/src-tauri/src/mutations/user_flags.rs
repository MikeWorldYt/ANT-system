// Adds or removes the user flag "data" on a Node folder, the only flag a user can set.
use std::path::Path;

use super::{finish, guard_frozen, load_synced, locate, Mutation};
use crate::ant_core::tree_types::{flags_in, type_base};
use crate::config::{DATA_FLAG, FOOTPRINT_FILENAME};
use crate::error::{FmError, R};
use crate::fs_safe::StorageLock;
use crate::index::footprint::mirror_flags;
use crate::tree::tree_store::{self, get_node_mut};

pub fn set_user_flag(root: &Path, rel: &[String], on: bool) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced(root)?;
    let target = locate(root, &data, rel)?;
    guard_frozen(rel, &target.node_type)?;
    if target.ghost {
        return Err(FmError::GhostNotAllowed { op: "set flag".to_string() });
    }
    if rel.len() == 1 {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "root folders are always main".to_string() });
    }
    if type_base(&target.node_type) != "NODE" {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "only Node folders take the data flag".to_string() });
    }
    let node = get_node_mut(&mut data, rel).ok_or_else(|| FmError::NotFound { rel_parts: rel.to_vec() })?;
    let mut flags = flags_in(node);
    flags.retain(|f| f != DATA_FLAG);
    if on {
        flags.push(DATA_FLAG.to_string());
    }
    if flags.is_empty() {
        node.remove("flags");
    } else {
        node.insert("flags".into(), flags.join(",").into());
    }
    tree_store::save(root, &data)?;
    // The tree already holds the flag, so a failed mirror into dragpath.json is only reported.
    let mut warnings = Vec::new();
    if let Err(e) = mirror_flags(&target.abs, &flags) {
        warnings.push(format!("Could not update {FOOTPRINT_FILENAME}: {e}"));
    }
    Ok(finish(root, &data, synced, Some(rel.to_vec()), warnings))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use crate::tree::tree_store::get_node;

    fn flags_of(dir: &Path, p: &[String]) -> Option<String> {
        get_node(&yaml(dir), p).unwrap().get("flags").and_then(|v| v.as_str()).map(str::to_string)
    }

    #[test]
    fn data_flag_is_added_and_removed_on_nodes_only() {
        let dir = academic("user-flags");
        let bio = rel(&["100.ACADEMIC", "02.Bio"]);
        set_user_flag(dir.path(), &bio, true).unwrap();
        assert_eq!(flags_of(dir.path(), &bio).as_deref(), Some("data"));
        set_user_flag(dir.path(), &bio, true).unwrap();
        set_user_flag(dir.path(), &bio, false).unwrap();
        assert_eq!(flags_of(dir.path(), &bio), None);
        assert!(matches!(set_user_flag(dir.path(), &rel(&["100.ACADEMIC", "03.Ghost"]), true), Err(FmError::GhostNotAllowed { .. })));
        assert!(matches!(set_user_flag(dir.path(), &rel(&["100.ACADEMIC", "01.Math", "98.Math"]), true), Err(FmError::Reserved { .. }) | Err(FmError::Locked { .. })));
        assert!(matches!(set_user_flag(dir.path(), &rel(&["100.ACADEMIC"]), true), Err(FmError::InvalidPath { .. })));
    }

    #[test]
    fn the_flag_follows_an_existing_dragpath_without_removing_it() {
        let dir = academic("user-flags-dragpath");
        let bio = dir.path().join("100.ACADEMIC/02.Bio");
        std::fs::create_dir_all(bio.join(".ant")).unwrap();
        std::fs::write(bio.join(".ant/dragpath.json"), "{\"id\": \"ABCDEF1234567\"}\n").unwrap();
        set_user_flag(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), true).unwrap();
        let text = std::fs::read_to_string(bio.join(".ant/dragpath.json")).unwrap();
        assert!(text.contains("\"id\": \"ABCDEF1234567\"") && text.contains("\"data\""), "{text}");
    }
}
