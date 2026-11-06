// Rebuilds every 998.Review shortcut from the tree, on refresh and when the per-branch update cannot be trusted.
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde_yaml::Mapping;

use crate::domain::names::support_kind;
use crate::domain::shortcut_names::review_shortcut_name;

#[cfg(windows)]
use crate::infra::fs_safe::is_link;
use crate::platform::shortcuts::{create_shortcuts, shortcut_targets};
use crate::services::tree::depth;
use crate::services::tree::tree_store::get_node;
use crate::services::tree::review_links::{is_locked, owners, review_dir, rev_folder};

pub(crate) type LinkTargets = Result<Vec<String>, String>;

pub fn rebuild(root: &Path, data: &Mapping) -> Vec<String> {
    rebuild_using(root, data, create_shortcuts, shortcut_targets)
}

// Stands in for the shell: each test .lnk file holds its target path as text.
#[cfg(test)]
pub(crate) fn text_targets(links: &[PathBuf]) -> LinkTargets {
    links.iter().map(|link| fs::read_to_string(link).map_err(|e| e.to_string())).collect()
}

#[cfg(test)]
pub(crate) fn rebuild_with(
    root: &Path,
    data: &Mapping,
    create: impl FnOnce(&[(PathBuf, PathBuf)]) -> Vec<String>,
) -> Vec<String> {
    rebuild_using(root, data, create, text_targets)
}

fn rebuild_using(
    root: &Path,
    data: &Mapping,
    create: impl FnOnce(&[(PathBuf, PathBuf)]) -> Vec<String>,
    read_targets: impl FnOnce(&[PathBuf]) -> LinkTargets,
) -> Vec<String> {
    let review_dir = match review_dir(root) {
        Ok(Some(dir)) => dir,
        Ok(None) => return Vec::new(),
        Err(failure) => return vec![failure],
    };
    let limit = match depth::limit(root) {
        Ok(limit) => limit,
        Err(e) => return vec![e.to_string()],
    };
    let mut failures = remove_old_links(root, &review_dir, read_targets);
    let mut found = Vec::new();
    owners(data, &[], &mut found, limit);
    let mut pairs = Vec::new();
    for owner in &found {
        let rev_abs = rev_folder(root, owner);
        if rev_abs.is_dir() {
            match review_shortcut_name(owner, limit) {
                Ok(link) => pairs.push((rev_abs, review_dir.join(link))),
                Err(e) => failures.push(e.to_string()),
            }
        }
    }
    failures.extend(create(&pairs));
    failures
}

// Owners of one branch under the full walk's rules: no locked node on the way and nothing past the depth limit.
pub(super) fn branch_owners(data: &Mapping, branch: &[String], node: &Mapping, limit: usize) -> Vec<Vec<String>> {
    let open = |depth: usize| get_node(data, &branch[..depth]).is_some_and(|n| !is_locked(n));
    if branch.is_empty() || branch.len() > limit || is_locked(node) || !(1..branch.len()).all(open) {
        return Vec::new();
    }
    let mut found = vec![branch.to_vec()];
    if branch.len() < limit {
        owners(node, branch, &mut found, limit);
    }
    found
}

// A .lnk is the app's only when it opens a review folder of this storage; any other is the user's and stays.
pub(crate) fn own_shortcuts(root: &Path, review_dir: &Path, links: Vec<PathBuf>, read_targets: impl FnOnce(&[PathBuf]) -> LinkTargets) -> Result<Vec<PathBuf>, String> {
    if links.is_empty() {
        return Ok(links);
    }
    let targets = read_targets(&links)?;
    Ok(links.into_iter().zip(targets).filter(|(_, target)| opens_review(root, review_dir, target)).map(|(link, _)| link).collect())
}

// Windows paths ignore case, and a shortcut keeps whatever case the storage path had when it was made.
fn opens_review(root: &Path, review_dir: &Path, target: &str) -> bool {
    let lower = |path: &Path| PathBuf::from(path.to_string_lossy().to_lowercase());
    let target = Path::new(target);
    targets_storage(&lower(root), &lower(review_dir), &lower(target)) && target.file_name().is_some_and(|name| support_kind(&name.to_string_lossy()) == Some("REV"))
}

fn remove_old_links(root: &Path, review_dir: &Path, read_targets: impl FnOnce(&[PathBuf]) -> LinkTargets) -> Vec<String> {
    let entries = match fs::read_dir(review_dir) {
        Ok(entries) => entries,
        Err(e) => return vec![format!("Could not list {}: {e}", review_dir.display())],
    };
    let mut failures = Vec::new();
    let mut stale_links = Vec::new();
    let mut shortcuts = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                failures.push(format!("Could not list {}: {e}", review_dir.display()));
                continue;
            }
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        // An online-only file is the user's whatever its name, and reading it would download it.
        #[cfg(windows)]
        if name.to_lowercase().ends_with(".lnk") && entry.metadata().is_ok_and(|meta| meta.is_file() && !is_link(&meta)) {
            shortcuts.push(entry.path());
        }
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
        #[cfg(not(windows))]
        if stale {
            stale_links.push(entry.path());
        }
    }
    match own_shortcuts(root, review_dir, shortcuts, read_targets) {
        Ok(own) => stale_links.extend(own),
        Err(e) => failures.push(format!("Could not read the review shortcuts: {e}")),
    }
    for link in stale_links {
        if let Err(e) = fs::remove_file(&link) {
            failures.push(format!("Could not remove {}: {e}", link.display()));
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
        let old = dir.path().join("100.ACADEMIC/98.Gone").to_string_lossy().into_owned();
        fs::write(review.join("old.lnk"), &old).unwrap();
        fs::write(review.join("OLD2.LNK"), &old).unwrap();
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
            (a.join("01.Math/98.Math"), review.join("101.00.Math")),
            (a.join("01.Math/PJ.Thesis/98.Thesis"), review.join("101.PJ.Thesis")),
        ];
        assert_eq!(seen, expected);
    }

    #[cfg(windows)]
    #[test]
    fn rebuild_keeps_a_shortcut_that_does_not_open_a_review_folder() {
        let dir = storage("review-own", &["998.Review", "100.A/REV.A"], "100.A: {type: NODE-L1}\n");
        let review = dir.path().join("998.Review");
        let stale = dir.path().join("100.A/REV.Gone").to_string_lossy().into_owned();
        fs::write(review.join("stale.lnk"), stale).unwrap();
        fs::write(review.join("mine.lnk"), "C:/Users/me/report.pdf").unwrap();
        fs::write(review.join("inside.lnk"), dir.path().join("100.A/report.pdf").to_string_lossy().as_bytes()).unwrap();
        assert!(rebuild_with(dir.path(), &yaml(dir.path()), |_| Vec::new()).is_empty());
        assert!(review.join("mine.lnk").is_file() && review.join("inside.lnk").is_file());
        assert!(!review.join("stale.lnk").exists());
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
        let link = dir.path().join("998.Review/101.00.Café.lnk");
        assert!(link.is_file());
        let target = dir.path().join("100.AÑO/01.Café/98.Café").to_string_lossy().replace('/', "\\");
        assert_eq!(shortcut_targets(std::slice::from_ref(&link)).unwrap(), [target]);
        assert!(rebuild(dir.path(), &yaml(dir.path())).is_empty());
        assert!(link.is_file());
    }
}
