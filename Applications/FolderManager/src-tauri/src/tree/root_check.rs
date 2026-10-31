// Level-1 integrity: root folders outside the category format, and the review folders they can move into.
use std::path::Path;

use serde::Serialize;

use crate::ant_core::names::{is_main_category, is_valid_root_name, list_subdirs};
use crate::ant_core::tree_types::type_base;
use crate::config::TypeBase;
use crate::error::R;
use crate::tree::tree_store::infer_type;

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RootCheck {
    pub invalid: Vec<String>,
    pub reviews: Vec<Vec<String>>,
}

pub fn check_root(root: &Path) -> R<RootCheck> {
    let dirs = list_subdirs(root)?;
    let invalid: Vec<String> = dirs.iter().filter(|name| !is_valid_root_name(name)).cloned().collect();
    // A clean storage costs one listing; the categories are only read when something has to move.
    let reviews = if invalid.is_empty() { Vec::new() } else { review_targets(root, &dirs)? };
    Ok(RootCheck { invalid, reviews })
}

fn review_targets(root: &Path, dirs: &[String]) -> R<Vec<Vec<String>>> {
    let mut targets = Vec::new();
    for category in dirs.iter().filter(|name| is_main_category(name)) {
        for name in list_subdirs(&root.join(category))? {
            if type_base(&infer_type(&name, 1)) == TypeBase::Rev.as_str() {
                targets.push(vec![category.clone(), name]);
            }
        }
    }
    Ok(targets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{rel, TempDir};

    #[test]
    fn only_root_folders_outside_the_format_are_reported_with_the_category_review_folders() {
        let dir = TempDir::new("root-check");
        for d in ["100.WORK/98.WORK", "100.WORK/01.Plain", "200.HOME/98.HOME", "998.Review", "999.Settings", ".ant", ".ant-trash", "Marcos/98.Marcos", "574.Notes", "New Folder"] {
            dir.mkdir(d);
        }
        let check = check_root(dir.path()).unwrap();
        assert_eq!(check.invalid, ["574.Notes", "Marcos", "New Folder"]);
        assert_eq!(check.reviews, [rel(&["100.WORK", "98.WORK"]), rel(&["200.HOME", "98.HOME"])]);
    }

    #[test]
    fn a_valid_root_skips_the_review_listing() {
        let dir = TempDir::new("root-check-clean");
        for d in ["100.WORK/98.WORK", "999.Settings", ".ant"] {
            dir.mkdir(d);
        }
        assert_eq!(check_root(dir.path()).unwrap(), RootCheck { invalid: Vec::new(), reviews: Vec::new() });
    }
}
