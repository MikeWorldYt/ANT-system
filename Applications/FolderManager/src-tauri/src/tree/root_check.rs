// Folders that break the id format, at the root and in every level the database covers, each with the review folders it can move into.
use std::path::Path;

use serde::Serialize;
use serde_yaml::{Mapping, Value};

use crate::ant_core::names::{has_node_id, is_main_category, is_valid_root_name, list_subdirs};
use crate::ant_core::tree_types::type_base;
use crate::config::TypeBase;
use crate::error::R;
use crate::tree::depth;
use crate::tree::tree_node::is_reserved;
use crate::tree::tree_store::{self, infer_type};

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Invalid {
    pub rel_parts: Vec<String>,
    pub review: Vec<Vec<String>>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RootCheck {
    pub invalid: Vec<Invalid>,
}

pub fn check_root(root: &Path) -> R<RootCheck> {
    let limit = depth::limit(root)?;
    let data = tree_store::load(root)?;
    let mut invalid = Vec::new();
    walk(root, root, Some(&data), &mut Vec::new(), limit, &mut invalid)?;
    Ok(RootCheck { invalid })
}

// Only node folders are checked; review, app and workboard folders keep their own names.
fn walk(root: &Path, dir: &Path, node: Option<&Mapping>, rel: &mut Vec<String>, limit: usize, out: &mut Vec<Invalid>) -> R<()> {
    for name in list_subdirs(dir)? {
        if rel.is_empty() && is_reserved(std::slice::from_ref(&name)) {
            continue;
        }
        let child = node.and_then(|n| n.get(name.as_str())).and_then(Value::as_mapping);
        let kind = child.and_then(|c| c.get("type")).and_then(Value::as_str).map_or_else(|| infer_type(&name, rel.len()), str::to_string);
        if type_base(&kind) != "NODE" {
            continue;
        }
        let valid = if rel.is_empty() { is_valid_root_name(&name) } else { has_node_id(&name) };
        rel.push(name.clone());
        if !valid {
            out.push(Invalid { rel_parts: rel.clone(), review: review_for(root, dir, rel)? });
        }
        if rel.len() < limit {
            walk(root, &dir.join(&name), child, rel, limit, out)?;
        }
        rel.pop();
    }
    Ok(())
}

// A root folder can go into any category's review folder; a deeper one only into the review folder beside it.
fn review_for(root: &Path, dir: &Path, rel: &[String]) -> R<Vec<Vec<String>>> {
    if rel.len() == 1 {
        return category_reviews(root);
    }
    let sibling = list_subdirs(dir)?.into_iter().find(|name| type_base(&infer_type(name, rel.len() - 1)) == TypeBase::Rev.as_str());
    Ok(sibling
        .map(|name| {
            let mut parts = rel[..rel.len() - 1].to_vec();
            parts.push(name);
            vec![parts]
        })
        .unwrap_or_default())
}

fn category_reviews(root: &Path) -> R<Vec<Vec<String>>> {
    let mut targets = Vec::new();
    for category in list_subdirs(root)?.iter().filter(|name| is_main_category(name)) {
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
    fn folders_outside_the_id_format_list_the_review_folder_they_can_move_into() {
        let dir = TempDir::new("root-check");
        for d in ["100.WORK/98.WORK", "100.WORK/01.Plain", "100.WORK/Physics", "200.HOME/98.HOME", "998.Review", "999.Settings", ".ant", ".ant-trash", "Marcos/98.Marcos", "574.Notes", "New Folder"] {
            dir.mkdir(d);
        }
        let check = check_root(dir.path()).unwrap();
        let found: Vec<Vec<String>> = check.invalid.iter().map(|i| i.rel_parts.clone()).collect();
        assert_eq!(found, [rel(&["100.WORK", "Physics"]), rel(&["574.Notes"]), rel(&["Marcos"]), rel(&["New Folder"])]);
        assert_eq!(check.invalid[0].review, [rel(&["100.WORK", "98.WORK"])]);
        assert_eq!(check.invalid[1].review, [rel(&["100.WORK", "98.WORK"]), rel(&["200.HOME", "98.HOME"])]);
    }

    #[test]
    fn a_clean_storage_reports_nothing_and_levels_past_the_database_depth_are_not_read() {
        let dir = TempDir::new("root-check-clean");
        for d in ["100.WORK/98.WORK", "100.WORK/01.Plain/02.Deep/Inside", "900.Sys/Stray", "999.Settings/00.Core", ".ant"] {
            dir.mkdir(d);
        }
        assert_eq!(check_root(dir.path()).unwrap(), RootCheck { invalid: Vec::new() });
    }
}
