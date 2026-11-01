// Folders that break the id format, at the root and in every level the database covers, each with the review folders it can move into.
use std::path::Path;

use serde::Serialize;
use serde_yaml::{Mapping, Value};

use crate::ant_core::names::{has_node_id, is_main_category, is_track_name, is_valid_root_name, is_workboard_name, list_subdirs};
use crate::ant_core::tree_types::type_base;
use crate::config::{TypeBase, DATA_DIR, INFO_FILENAME, WKB_IDENTIFIERS};
use crate::error::R;
use crate::tree::depth;
use crate::tree::tree_node::is_reserved;
use crate::tree::tree_store::{self, infer_type};

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Invalid {
    pub rel_parts: Vec<String>,
    pub review: Vec<Vec<String>>,
    pub workboard: bool,
    pub tracker: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RootCheck {
    pub invalid: Vec<Invalid>,
}

pub fn check_root(root: &Path) -> R<RootCheck> {
    let limit = depth::limit(root)?;
    let mut data = tree_store::load(root)?;
    let changed = tree_store::sync(root, &mut data)? | tree_store::reconcile_portfolios_all(root, &mut data)?;
    if changed {
        tree_store::save(root, &data)?;
    }
    let mut invalid = Vec::new();
    walk(root, root, Some(&data), &mut Vec::new(), limit, &mut invalid)?;
    Ok(RootCheck { invalid })
}

// Node and workboard folders are checked; review and app folders keep their own names. Workboards hold no nodes, so they are not entered.
fn walk(root: &Path, dir: &Path, node: Option<&Mapping>, rel: &mut Vec<String>, limit: usize, out: &mut Vec<Invalid>) -> R<()> {
    for name in list_subdirs(dir)? {
        if rel.is_empty() && is_reserved(std::slice::from_ref(&name)) {
            continue;
        }
        let child = node.and_then(|n| n.get(name.as_str())).and_then(Value::as_mapping);
        let kind = child
            .and_then(|c| c.get("type"))
            .and_then(Value::as_str)
            .filter(|kind| !(name.starts_with("00.") && type_base(kind) == "NODE"))
            .map_or_else(|| if name.starts_with("00.") { "PORTF".to_string() } else { infer_type(&name, rel.len()) }, str::to_string);
        let base = type_base(&kind);
        if base != "NODE" && base != "WKB" {
            continue;
        }
        let valid = match base {
            "WKB" => rel.is_empty() || is_track_name(&name),
            _ if rel.is_empty() => is_valid_root_name(&name),
            _ => has_node_id(&name),
        };
        rel.push(name.clone());
        if !valid {
            let workboard = base == "WKB" || is_workboard_name(&name) || has_info(&dir.join(&name));
            let tracker = WKB_IDENTIFIERS.iter().find(|t| name.starts_with(**t)).map(|t| t.trim_end_matches('.').to_string());
            out.push(Invalid { rel_parts: rel.clone(), review: review_for(root, dir, rel)?, workboard, tracker });
        }
        if base == "NODE" && rel.len() < limit {
            walk(root, &dir.join(&name), child, rel, limit, out)?;
        }
        rel.pop();
    }
    Ok(())
}

fn has_info(folder: &Path) -> bool {
    folder.join(DATA_DIR).join(INFO_FILENAME).is_file()
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
    use crate::test_support::{rel, storage, TempDir};

    #[test]
    fn a_workboard_outside_the_track_format_is_flagged_with_its_tracker() {
        let dir = storage(
            "root-check-wkb",
            &["100.WORK/01.New-Node", "100.WORK/PJ.1007_Prueba-7", "100.WORK/PJ.001_Fine"],
            "100.WORK:\n  type: NODE-L1\n  01.New-Node: {type: WKB-L2}\n  PJ.1007_Prueba-7: {type: NODE-L2}\n  PJ.001_Fine: {type: WKB-L2}\n",
        );
        let check = check_root(dir.path()).unwrap();
        let found: Vec<Vec<String>> = check.invalid.iter().map(|i| i.rel_parts.clone()).collect();
        assert_eq!(found, [rel(&["100.WORK", "01.New-Node"]), rel(&["100.WORK", "PJ.1007_Prueba-7"])]);
        assert!(check.invalid[0].workboard && check.invalid[0].tracker.is_none());
        assert!(check.invalid[1].workboard);
        assert_eq!(check.invalid[1].tracker.as_deref(), Some("PJ"));
    }

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

    #[test]
    fn a_legacy_00_portfolio_keeps_its_name_and_becomes_a_portfolio() {
        let dir = storage(
            "root-check-legacy-portfolio",
            &["100.WORK/00.Porfolio/01.Notes"],
            "100.WORK:\n  type: NODE-L1\n  00.Porfolio: {type: NODE-L2}\n",
        );
        let result = check_root(dir.path()).unwrap();
        assert!(result.invalid.is_empty());
        assert!(dir.path().join("100.WORK/00.Porfolio/01.Notes").is_dir());
        assert_eq!(tree_store::get_type(&tree_store::load(dir.path()).unwrap(), &rel(&["100.WORK", "00.Porfolio"])), "PORTF-L2");
    }
}
