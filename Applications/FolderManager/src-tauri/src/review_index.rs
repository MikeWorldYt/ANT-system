// Rebuilds the 998.Review shortcuts from the tree; the command layer runs it after rename, move and delete.
use std::fs;
use std::path::{Path, PathBuf};

use serde_yaml::{Mapping, Value};

use crate::ant_core::shortcuts::{create_shortcuts, review_shortcut_name};
use crate::ant_core::tree_types::child_entries;
use crate::config::{MAX_DEPTH, REVIEW_FOLDER};
use crate::fs_safe::is_link;
use crate::names::editable_part;
use crate::tree_store::is_locked_type;

pub fn rebuild(root: &Path, data: &Mapping) -> Vec<String> {
    rebuild_with(root, data, create_shortcuts)
}

pub(crate) fn rebuild_with(
    root: &Path,
    data: &Mapping,
    create: impl FnOnce(&[(PathBuf, PathBuf)]) -> Vec<String>,
) -> Vec<String> {
    let review_dir = root.join(REVIEW_FOLDER);
    match fs::symlink_metadata(&review_dir) {
        Ok(meta) if meta.is_dir() && !is_link(&meta) => {}
        // Clearing .lnk files through a linked 998.Review would delete files outside the storage.
        Ok(meta) if is_link(&meta) => return vec![format!("{REVIEW_FOLDER} is a link; index not rebuilt")],
        _ => return Vec::new(),
    }
    let mut failures = remove_old_links(&review_dir);
    let mut pairs = Vec::new();
    walk(data, &[], root, &review_dir, &mut pairs, &mut failures);
    failures.extend(create(&pairs));
    failures
}

fn remove_old_links(review_dir: &Path) -> Vec<String> {
    let entries = match fs::read_dir(review_dir) {
        Ok(entries) => entries,
        Err(e) => return vec![format!("Could not list {}: {e}", review_dir.display())],
    };
    let mut failures = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                failures.push(format!("Could not list {}: {e}", review_dir.display()));
                continue;
            }
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        let regular_file = entry.file_type().is_ok_and(|t| t.is_file());
        if regular_file && name.to_lowercase().ends_with(".lnk") {
            if let Err(e) = fs::remove_file(entry.path()) {
                failures.push(format!("Could not remove {name}: {e}"));
            }
        }
    }
    failures
}

fn walk(
    node: &Mapping,
    parts: &[String],
    abs_dir: &Path,
    review_dir: &Path,
    pairs: &mut Vec<(PathBuf, PathBuf)>,
    failures: &mut Vec<String>,
) {
    let mut children: Vec<(&str, &Mapping)> = child_entries(node).collect();
    children.sort_by(|a, b| a.0.cmp(b.0));
    for (name, child) in children {
        if is_locked_type(child.get("type").and_then(Value::as_str).unwrap_or("")) {
            continue;
        }
        let mut child_parts = parts.to_vec();
        child_parts.push(name.to_string());
        let child_abs = abs_dir.join(name);
        let rev_abs = child_abs.join(format!("98.{}", editable_part(name)));
        if rev_abs.is_dir() {
            match review_shortcut_name(&child_parts, "") {
                Ok(link) => pairs.push((rev_abs, review_dir.join(link))),
                Err(e) => failures.push(e.to_string()),
            }
        }
        if child_parts.len() < MAX_DEPTH {
            walk(child, &child_parts, &child_abs, review_dir, pairs, failures);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit_actions::test_support::{storage, yaml};

    #[test]
    fn rebuild_clears_only_top_level_lnk_files_and_lists_review_folders() {
        let dir = storage(
            "review",
            &[
                "998.Review/keep.lnk",
                "100.ACADEMIC/98.ACADEMIC",
                "100.ACADEMIC/01.Math/98.Math",
                "100.ACADEMIC/01.Math/PJ.Thesis/98.Thesis",
                "100.ACADEMIC/02.Bio",
                "100.ACADEMIC/90.Res/98.Res",
            ],
            "100.ACADEMIC:\n  type: NODE-L1\n  01.Math:\n    type: NODE-L2\n    PJ.Thesis: {type: WKB}\n  \
             02.Bio: {type: NODE-L2}\n  90.Res: {type: RES}\n",
        );
        let review = dir.path().join("998.Review");
        fs::write(review.join("old.lnk"), "").unwrap();
        fs::write(review.join("OLD2.LNK"), "").unwrap();
        fs::write(review.join("notes.txt"), "").unwrap();
        fs::write(review.join("keep.lnk").join("inner.lnk"), "").unwrap();

        let mut seen = Vec::new();
        let failures = rebuild_with(dir.path(), &yaml(dir.path()), |pairs| {
            seen = pairs.to_vec();
            vec!["stub failure".to_string()]
        });
        assert_eq!(failures, ["stub failure"]);
        let mut left: Vec<String> = fs::read_dir(&review)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(left, ["keep.lnk", "notes.txt"]);
        assert!(review.join("keep.lnk/inner.lnk").is_file());

        let a = dir.path().join("100.ACADEMIC");
        let expected = vec![
            (a.join("98.ACADEMIC"), review.join("100.ACADEMIC")),
            (a.join("01.Math/98.Math"), review.join("101.Math")),
            (a.join("01.Math/PJ.Thesis/98.Thesis"), review.join("101.PJ.Thesis")),
        ];
        assert_eq!(seen, expected);
    }

    #[test]
    fn rebuild_without_review_folder_does_nothing() {
        let dir = storage("review-none", &["100.A/98.A"], "100.A: {type: NODE-L1}\n");
        let failures = rebuild_with(dir.path(), &yaml(dir.path()), |_| panic!("nothing to create"));
        assert!(failures.is_empty());
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "starts PowerShell and creates real .lnk files"]
    fn rebuild_creates_real_shortcuts() {
        let dir = storage(
            "review-real",
            &["998.Review", "100.AÑO/98.AÑO", "100.AÑO/01.Café/98.Café"],
            "100.AÑO:\n  type: NODE-L1\n  01.Café: {type: NODE-L2}\n",
        );
        let failures = rebuild(dir.path(), &yaml(dir.path()));
        assert!(failures.is_empty(), "{failures:?}");
        assert!(dir.path().join("998.Review/100.AÑO.lnk").is_file());
        assert!(dir.path().join("998.Review/101.Café.lnk").is_file());
    }
}
