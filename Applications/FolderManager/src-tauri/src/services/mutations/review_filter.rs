// Saves which categories the 998.Review filter hides, in that folder's own info.json.
use std::path::Path;

use serde_yaml::Value;

use super::{finish, load_synced_at, Mutation};
use crate::domain::config::REVIEW_FOLDER;
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::{validate_component, StorageLock};
use crate::infra::storage::info::write_info;

pub fn set_review_filter(root: &Path, hidden: &[String], favorites: bool) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    for category in hidden {
        validate_component(category)?;
        if category.contains(',') {
            return Err(FmError::InvalidPath { rel_parts: vec![category.clone()], reason: "A category name cannot hold a comma".to_string() });
        }
    }
    let (data, synced) = load_synced_at(root, &[])?;
    let hidden_value = (!hidden.is_empty()).then(|| Value::String(hidden.join(",")));
    let favorites_value = (!favorites).then(|| Value::String("off".to_string()));
    write_info(
        &root.join(REVIEW_FOLDER),
        &[(&["hidden"], "hidden", hidden_value), (&["favorites"], "favorites", favorites_value)],
    )?;
    let rel = vec![REVIEW_FOLDER.to_string()];
    // No selection: the user keeps the item they are on while the list filters around it.
    Ok(finish(root, &data, synced, None, Vec::new(), vec![rel]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::tree::review_links::hidden_categories;
    use crate::test_support::{names, storage};
    use crate::services::tree::tree_node::build_children;

    #[test]
    fn a_hidden_category_takes_its_links_off_the_review_list_until_it_is_shown_again() {
        let dir = storage(
            "review-filter",
            &["998.Review", "100.A/98.A", "200.B/98.B"],
            "100.A: {type: NODE-L1}\n200.B: {type: NODE-L1}\n",
        );
        std::fs::write(dir.path().join("998.Review/100.A.lnk"), "").unwrap();
        std::fs::write(dir.path().join("998.Review/200.B.lnk"), "").unwrap();
        let data = crate::test_support::yaml(dir.path());
        let review = ["998.Review".to_string()];
        assert_eq!(names(&dir.path().join("998.Review")).len(), 2);
        set_review_filter(dir.path(), &["200.B".to_string()], true).unwrap();
        assert_eq!(hidden_categories(dir.path()), ["200.B"]);
        let listed: Vec<String> = build_children(dir.path(), &data, &review).unwrap().into_iter().map(|n| n.name).collect();
        assert_eq!(listed, ["100.A"]);
        set_review_filter(dir.path(), &[], true).unwrap();
        assert!(hidden_categories(dir.path()).is_empty());
        let listed = build_children(dir.path(), &data, &review).unwrap();
        assert_eq!(listed.len(), 2);
    }

    #[test]
    fn a_starred_link_keeps_its_category_heading_under_a_hidden_category() {
        let dir = storage(
            "review-filter-heading",
            &["998.Review", "200.B/98.B", "200.B/03.C/98.C"],
            "200.B:\n  type: NODE-L1\n  03.C:\n    type: NODE-L2\n",
        );
        std::fs::write(dir.path().join("998.Review/200.B.lnk"), "").unwrap();
        let child = crate::domain::shortcut_names::review_shortcut_name(&["200.B".to_string(), "03.C".to_string()], 3).unwrap();
        std::fs::write(dir.path().join(format!("998.Review/{child}.lnk")), "").unwrap();
        crate::infra::link_list::set(dir.path(), crate::domain::config::LINK_FAVORITES_FILENAME, &child, true).unwrap();
        let data = crate::test_support::yaml(dir.path());
        let review = ["998.Review".to_string()];
        set_review_filter(dir.path(), &["200.B".to_string()], true).unwrap();
        let listed: Vec<String> = build_children(dir.path(), &data, &review).unwrap().into_iter().map(|n| n.name).collect();
        assert_eq!(listed, ["200.B", child.as_str()]);
    }

    #[test]
    fn starred_links_stay_listed_under_a_hidden_category_while_favorites_is_checked() {
        let dir = storage(
            "review-filter-favorites",
            &["998.Review", "100.A/98.A", "200.B/98.B"],
            "100.A: {type: NODE-L1}\n200.B: {type: NODE-L1}\n",
        );
        std::fs::write(dir.path().join("998.Review/100.A.lnk"), "").unwrap();
        std::fs::write(dir.path().join("998.Review/200.B.lnk"), "").unwrap();
        crate::infra::link_list::set(dir.path(), crate::domain::config::LINK_FAVORITES_FILENAME, "200.B", true).unwrap();
        let data = crate::test_support::yaml(dir.path());
        let review = ["998.Review".to_string()];
        set_review_filter(dir.path(), &["200.B".to_string()], true).unwrap();
        let listed: Vec<String> = build_children(dir.path(), &data, &review).unwrap().into_iter().map(|n| n.name).collect();
        assert_eq!(listed, ["100.A", "200.B"]);
        set_review_filter(dir.path(), &["200.B".to_string()], false).unwrap();
        let listed: Vec<String> = build_children(dir.path(), &data, &review).unwrap().into_iter().map(|n| n.name).collect();
        assert_eq!(listed, ["100.A"]);
    }

    #[test]
    fn a_category_name_with_a_comma_is_refused() {
        let dir = storage("review-filter-comma", &["998.Review"], "");
        assert!(set_review_filter(dir.path(), &["100.A,B".to_string()], true).is_err());
    }
}
