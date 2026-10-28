// Rebuilds every 998.Review shortcut from the tree, on refresh and when the per-branch update cannot be trusted.
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde_yaml::{Mapping, Value};

use crate::ant_core::shortcuts::{create_shortcuts, review_shortcut_name};
use crate::ant_core::tree_types::child_entries;
use crate::config::{MAX_DEPTH, REVIEW_FOLDER};
use crate::fs_safe::{is_link, join_rel};
use crate::ant_core::names::editable_part;
use crate::tree::tree_store::{get_node, is_locked_type};

pub fn rebuild(root: &Path, data: &Mapping) -> Vec<String> {
    rebuild_with(root, data, create_shortcuts)
}

pub(crate) fn rebuild_with(
    root: &Path,
    data: &Mapping,
    create: impl FnOnce(&[(PathBuf, PathBuf)]) -> Vec<String>,
) -> Vec<String> {
    let review_dir = match review_dir(root) {
        Ok(Some(dir)) => dir,
        Ok(None) => return Vec::new(),
        Err(failure) => return vec![failure],
    };
    let mut failures = remove_old_links(root, &review_dir);
    let mut found = Vec::new();
    owners(data, &[], &mut found);
    let mut pairs = Vec::new();
    for owner in &found {
        let rev_abs = rev_folder(root, owner);
        if rev_abs.is_dir() {
            match review_shortcut_name(owner, "") {
                Ok(link) => pairs.push((rev_abs, review_dir.join(link))),
                Err(e) => failures.push(e.to_string()),
            }
        }
    }
    failures.extend(create(&pairs));
    failures
}

pub(super) fn review_dir(root: &Path) -> Result<Option<PathBuf>, String> {
    let review_dir = root.join(REVIEW_FOLDER);
    match fs::symlink_metadata(&review_dir) {
        Ok(meta) if meta.is_dir() && !is_link(&meta) => Ok(Some(review_dir)),
        // Clearing .lnk files through a linked 998.Review would delete files outside the storage.
        Ok(meta) if is_link(&meta) => Err(format!("{REVIEW_FOLDER} is a link; index not rebuilt")),
        _ => Ok(None),
    }
}

pub(super) fn rev_folder(root: &Path, owner: &[String]) -> PathBuf {
    let editable = owner.last().map_or("", |name| editable_part(name));
    join_rel(root, owner).join(format!("98.{editable}"))
}

// Owners of one branch under the full walk's rules: no locked node on the way and nothing past MAX_DEPTH.
pub(super) fn branch_owners(data: &Mapping, branch: &[String], node: &Mapping) -> Vec<Vec<String>> {
    let open = |depth: usize| get_node(data, &branch[..depth]).is_some_and(|n| !is_locked(n));
    if branch.is_empty() || branch.len() > MAX_DEPTH || is_locked(node) || !(1..branch.len()).all(open) {
        return Vec::new();
    }
    let mut found = vec![branch.to_vec()];
    if branch.len() < MAX_DEPTH {
        owners(node, branch, &mut found);
    }
    found
}

fn remove_old_links(#[cfg_attr(windows, allow(unused_variables))] root: &Path, review_dir: &Path) -> Vec<String> {
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
        #[cfg(windows)]
        let stale = entry.file_type().is_ok_and(|t| t.is_file()) && name.to_lowercase().ends_with(".lnk");
        // Only symlinks into the storage are ours; dangling ones still resolve lexically.
        #[cfg(not(windows))]
        let stale = match entry.file_type() {
            Ok(t) if t.is_symlink() => match fs::read_link(entry.path()) {
                Ok(target) => targets_storage(root, review_dir, &target),
                Err(e) => {
                    failures.push(format!("Could not read link {name}: {e}"));
                    false
                }
            },
            _ => false,
        };
        if stale {
            if let Err(e) = fs::remove_file(entry.path()) {
                failures.push(format!("Could not remove {name}: {e}"));
            }
        }
    }
    failures
}

// A link into 998.Review itself may be the user's own, so only targets elsewhere in the storage count.
#[cfg_attr(windows, allow(dead_code))]
pub(super) fn targets_storage(root: &Path, review_dir: &Path, target: &Path) -> bool {
    let resolved = normalize(&review_dir.join(target));
    let root = normalize(root);
    resolved != root && resolved.starts_with(&root) && !resolved.starts_with(normalize(review_dir))
}

fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(out.components().next_back(), Some(Component::Normal(_))) {
                    out.pop();
                }
            }
            other => out.push(other),
        }
    }
    out
}

fn is_locked(node: &Mapping) -> bool {
    is_locked_type(node.get("type").and_then(Value::as_str).unwrap_or(""))
}

// Sorted pre-order, so the full rebuild always hands its pairs over in the same order.
fn owners(node: &Mapping, parts: &[String], found: &mut Vec<Vec<String>>) {
    let mut children: Vec<(&str, &Mapping)> = child_entries(node).collect();
    children.sort_by(|a, b| a.0.cmp(b.0));
    for (name, child) in children {
        if is_locked(child) {
            continue;
        }
        let mut child_parts = parts.to_vec();
        child_parts.push(name.to_string());
        found.push(child_parts.clone());
        if child_parts.len() < MAX_DEPTH {
            owners(child, &child_parts, found);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{storage, yaml};

    #[cfg(windows)]
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
    fn targets_storage_accepts_only_links_into_the_storage() {
        let root = Path::new("/st");
        let review = root.join("998.Review");
        assert!(targets_storage(root, &review, Path::new("/st/100.A/98.A")));
        assert!(targets_storage(root, &review, Path::new("../100.A/01.B/98.B")));
        assert!(targets_storage(root, &review, Path::new("/st/./gone/../100.A/98.A")));
        assert!(!targets_storage(root, &review, Path::new("/home/user/notes")));
        assert!(!targets_storage(root, &review, Path::new("../../st2/98.A")));
        assert!(!targets_storage(root, &review, Path::new("../../../../st/../etc")));
        assert!(!targets_storage(root, &review, Path::new("..")));
        assert!(!targets_storage(root, &review, Path::new("notes.txt")));
        assert!(!targets_storage(root, &review, Path::new("/st/998.Review/x")));
    }

    #[cfg(unix)]
    #[test]
    fn rebuild_removes_only_symlinks_into_the_storage() {
        use std::os::unix::fs::symlink;
        let dir = storage("review-unix", &["998.Review", "100.A/98.A"], "100.A: {type: NODE-L1}
");
        let review = dir.path().join("998.Review");
        symlink(dir.path().join("100.A/98.A"), review.join("100.A")).unwrap();
        symlink(dir.path().join("100.A/gone"), review.join("dangling")).unwrap();
        symlink("/tmp", review.join("elsewhere")).unwrap();
        fs::write(review.join("notes.txt"), "").unwrap();

        let failures = rebuild_with(dir.path(), &yaml(dir.path()), |_| Vec::new());
        assert!(failures.is_empty(), "{failures:?}");
        let mut left: Vec<String> = fs::read_dir(&review)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(left, ["elsewhere", "notes.txt"]);
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
