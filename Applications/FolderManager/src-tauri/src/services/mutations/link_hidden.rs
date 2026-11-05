// Hidden review links are listed in the app's data folder like starred ones, so the owner folder is never touched.
use std::path::Path;

use super::{finish, scope, load_synced_at, parent_of, Mutation};
use crate::domain::config::LINK_HIDDEN_FILENAME;
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::StorageLock;
use crate::infra::link_list;
use crate::services::tree::review_links;

pub fn set_link_hidden(root: &Path, rel: &[String], on: bool) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (data, synced) = load_synced_at(root, parent_of(rel))?;
    let link = match rel {
        [_, link] if review_links::linked_owner(root, &data, rel)?.is_some() => link,
        _ => return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "only review links can be hidden".to_string() }),
    };
    link_list::set(root, LINK_HIDDEN_FILENAME, link, on)?;
    Ok(finish(root, &data, false, Some(rel.to_vec()), Vec::new(), scope(&synced, vec![rel.to_vec()])))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

    #[test]
    fn a_review_link_is_listed_in_the_link_hidden_file_until_shown_again() {
        let dir = storage("link-hidden", &["998.Review", "100.A/98.A"], "100.A: {type: NODE-L1}\n");
        std::fs::write(dir.path().join("998.Review/100.A.lnk"), "").unwrap();
        let link = rel(&["998.Review", "100.A"]);
        set_link_hidden(dir.path(), &link, true).unwrap();
        assert_eq!(crate::infra::link_list::read(dir.path(), LINK_HIDDEN_FILENAME).unwrap(), ["100.A"]);
        set_link_hidden(dir.path(), &link, false).unwrap();
        assert!(crate::infra::link_list::read(dir.path(), LINK_HIDDEN_FILENAME).unwrap().is_empty());
    }

    #[test]
    fn only_review_links_can_be_hidden() {
        let dir = academic("link-hidden-folder");
        assert!(set_link_hidden(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), true).is_err());
    }
}
