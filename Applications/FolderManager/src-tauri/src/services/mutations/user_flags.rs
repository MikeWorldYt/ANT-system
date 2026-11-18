// Adds or removes a user flag: "data" lets a Node folder hold files, "autom" gives any folder the automations.
use std::path::Path;

use super::{editable_target, finish, scope, load_synced_at, parent_of, Mutation};
use crate::domain::tree_types::{flags_in, type_base};
use crate::domain::config::{AUTOM_FLAG, DATA_FLAG, FOOTPRINT_FILENAME};
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::StorageLock;
use crate::infra::footprint_file::mirror_flags;
use crate::services::tree::tree_store::{self, get_node_mut};

pub fn set_user_flag(root: &Path, rel: &[String], flag: &str, on: bool) -> R<Mutation> {
    if ![DATA_FLAG, AUTOM_FLAG].contains(&flag) {
        return Err(FmError::Internal { message: format!("unknown user flag {flag}") });
    }
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced_at(root, parent_of(rel))?;
    let target = editable_target(root, &data, rel, "set flag")?;
    if rel.len() == 1 {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "root folders are always main".to_string() });
    }
    if flag == DATA_FLAG && type_base(&target.node_type) != "NODE" {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "only Node folders take the data flag".to_string() });
    }
    let node = get_node_mut(&mut data, rel).ok_or_else(|| FmError::NotFound { rel_parts: rel.to_vec() })?;
    let mut flags = flags_in(node);
    flags.retain(|f| f != flag);
    if on {
        flags.push(flag.to_string());
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
    Ok(finish(root, &data, false, Some(rel.to_vec()), warnings, scope(&synced, vec![rel.to_vec()])))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use crate::services::tree::tree_store::get_node;

    fn flags_of(dir: &Path, p: &[String]) -> Option<String> {
        get_node(&yaml(dir), p).unwrap().get("flags").and_then(|v| v.as_str()).map(str::to_string)
    }

    #[test]
    fn data_flag_is_added_and_removed_on_nodes_only() {
        let dir = academic("user-flags");
        let bio = rel(&["100.ACADEMIC", "02.Bio"]);
        set_user_flag(dir.path(), &bio, DATA_FLAG, true).unwrap();
        assert_eq!(flags_of(dir.path(), &bio).as_deref(), Some("data"));
        set_user_flag(dir.path(), &bio, DATA_FLAG, true).unwrap();
        set_user_flag(dir.path(), &bio, DATA_FLAG, false).unwrap();
        assert_eq!(flags_of(dir.path(), &bio), None);
        assert!(matches!(set_user_flag(dir.path(), &rel(&["100.ACADEMIC", "03.Ghost"]), DATA_FLAG, true), Err(FmError::GhostNotAllowed { .. })));
        assert!(matches!(set_user_flag(dir.path(), &rel(&["100.ACADEMIC", "01.Math", "98.Math"]), DATA_FLAG, true), Err(FmError::Reserved { .. }) | Err(FmError::Locked { .. })));
        assert!(matches!(set_user_flag(dir.path(), &rel(&["100.ACADEMIC"]), DATA_FLAG, true), Err(FmError::InvalidPath { .. })));
    }

    #[test]
    fn autom_turns_on_the_automations_of_any_editable_folder() {
        let dir = academic("user-flags-autom");
        let bio = rel(&["100.ACADEMIC", "02.Bio"]);
        assert!(crate::services::mutations::set_review_automation(dir.path(), &bio, "reduce", true).is_err());
        set_user_flag(dir.path(), &bio, AUTOM_FLAG, true).unwrap();
        crate::services::mutations::set_review_automation(dir.path(), &bio, "reduce", true).unwrap();
        assert!(flags_of(dir.path(), &bio).unwrap().contains(AUTOM_FLAG));
    }

    #[test]
    fn the_flag_follows_an_existing_dragpath_without_removing_it() {
        let dir = academic("user-flags-dragpath");
        let bio = dir.path().join("100.ACADEMIC/02.Bio");
        std::fs::create_dir_all(bio.join(".ant")).unwrap();
        std::fs::write(bio.join(".ant/dragpath.json"), "{\"id\": \"ABCDEF1234567\"}\n").unwrap();
        set_user_flag(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), DATA_FLAG, true).unwrap();
        let text = std::fs::read_to_string(bio.join(".ant/dragpath.json")).unwrap();
        assert!(text.contains("\"id\": \"ABCDEF1234567\"") && text.contains("\"data\""), "{text}");
    }
}
