// Favorites skip the frozen-type guard: every folder but the reserved roots can be starred.
use std::path::Path;

use serde_yaml::Value;

use super::{finish, load_synced_at, locate, parent_of, register_ids, Mutation};
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::StorageLock;
use crate::domain::config::LINK_FAVORITES_FILENAME;
use crate::infra::link_list;
use crate::services::tree::review_links;
use crate::infra::storage::info::write_info;
use crate::services::tree::tree_node::is_reserved;
use crate::services::tree::tree_store;

// A favorite is a flag in the folder's own info.json, so it travels with the folder and is searchable.
pub fn set_favorite(root: &Path, rel: &[String], on: bool) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced_at(root, parent_of(rel))?;
    // A review link has no folder of its own, so its star lives in the app's data folder.
    if let [_, link] = rel {
        if review_links::linked_owner(root, &data, rel)?.is_some() {
            link_list::set(root, LINK_FAVORITES_FILENAME, link, on)?;
            return Ok(finish(root, &data, synced, Some(rel.to_vec()), Vec::new(), vec![rel.to_vec()]));
        }
    }
    let target = locate(root, &data, rel)?;
    if is_reserved(rel) {
        return Err(FmError::Reserved { rel_parts: rel.to_vec() });
    }
    if target.ghost {
        return Err(FmError::GhostNotAllowed { op: "favorite".to_string() });
    }
    write_info(&target.abs, &[(&["favorite"], "favorite", on.then_some(Value::Bool(true)))])?;
    let mut warnings = Vec::new();
    let moved = register_ids(root, &mut data, &mut warnings);
    tree_store::save(root, &data)?;
    Ok(finish(root, &data, synced || moved, Some(rel.to_vec()), warnings, vec![rel.to_vec()]))
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

    #[test]
    fn a_review_link_is_starred_in_the_link_favorites_file() {
        let dir = storage("favorite-link", &["998.Review", "100.A/98.A"], "100.A: {type: NODE-L1}\n");
        std::fs::write(dir.path().join("998.Review/100.A.lnk"), "").unwrap();
        let link = rel(&["998.Review", "100.A"]);
        set_favorite(dir.path(), &link, true).unwrap();
        assert_eq!(link_list::read(dir.path(), LINK_FAVORITES_FILENAME).unwrap(), ["100.A"]);
        set_favorite(dir.path(), &link, false).unwrap();
        assert!(link_list::read(dir.path(), LINK_FAVORITES_FILENAME).unwrap().is_empty());
    }
}
