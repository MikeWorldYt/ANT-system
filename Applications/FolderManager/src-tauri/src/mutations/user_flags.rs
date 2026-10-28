// Adds or removes the user flag "data" on a Node folder, the only flag a user can set.
use std::path::Path;

use super::{finish, guard_frozen, load_synced, locate, Mutation};
use crate::ant_core::tree_types::{flags_in, type_base};
use crate::config::DATA_FLAG;
use crate::error::{FmError, R};
use crate::fs_safe::StorageLock;
use crate::tree::tree_store::{self, get_node_mut};

pub fn set_user_flag(root: &Path, rel: &[String], on: bool) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced(root)?;
    let target = locate(root, &data, rel)?;
    guard_frozen(rel, &target.node_type)?;
    if target.ghost {
        return Err(FmError::GhostNotAllowed { op: "set flag".to_string() });
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
    Ok(finish(root, &data, synced, Some(rel.to_vec()), Vec::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use crate::tree::tree_store::get_node;

    #[test]
    fn data_flag_is_added_and_removed_on_nodes_only() {
        let dir = academic("user-flags");
        let bio = rel(&["100.ACADEMIC", "02.Bio"]);
        set_user_flag(dir.path(), &bio, true).unwrap();
        assert_eq!(get_node(&yaml(dir.path()), &bio).unwrap().get("flags").and_then(|v| v.as_str()), Some("data"));
        set_user_flag(dir.path(), &bio, true).unwrap();
        set_user_flag(dir.path(), &bio, false).unwrap();
        assert!(get_node(&yaml(dir.path()), &bio).unwrap().get("flags").is_none());
        assert!(matches!(set_user_flag(dir.path(), &rel(&["100.ACADEMIC", "03.Ghost"]), true), Err(FmError::GhostNotAllowed { .. })));
        assert!(matches!(set_user_flag(dir.path(), &rel(&["100.ACADEMIC", "01.Math", "98.Math"]), true), Err(FmError::Reserved { .. }) | Err(FmError::Locked { .. })));
    }
}
