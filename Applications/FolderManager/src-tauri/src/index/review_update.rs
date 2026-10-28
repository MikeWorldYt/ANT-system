// Updates only the 998.Review shortcuts of the branches a mutation touched; an Err sends the caller to the full rebuild.
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde_yaml::Mapping;

use super::review_index::{branch_owners, rev_folder};
use crate::ant_core::shortcuts::review_shortcut_name;
use crate::ant_core::tree_types::child_entries;
use crate::tree::tree_store::get_node;

pub(crate) fn update_with(
    root: &Path,
    review_dir: &Path,
    data: &Mapping,
    before: &[(Vec<String>, Mapping)],
    touched: &[Vec<String>],
    create: impl FnOnce(&[(PathBuf, PathBuf)]) -> Vec<String>,
) -> Result<Vec<String>, String> {
    let old: BTreeSet<Vec<String>> = before.iter().flat_map(|(rel, node)| branch_owners(data, rel, node)).collect();
    let new: BTreeSet<Vec<String>> = touched
        .iter()
        .filter_map(|rel| get_node(data, rel).map(|node| branch_owners(data, rel, node)))
        .flatten()
        .collect();
    let mut failures = Vec::new();
    let mut fresh = Vec::new();
    for owner in &new {
        let target = rev_folder(root, owner);
        if target.is_dir() {
            match review_shortcut_name(owner, "") {
                Ok(name) => fresh.push((name, target)),
                Err(e) => failures.push(e.to_string()),
            }
        }
    }
    // An owner without a valid name never got a link, so there is nothing of it to remove.
    let stale: Vec<String> = old.iter().filter_map(|owner| review_shortcut_name(owner, "").ok()).collect();
    check_unshared(root, data, &stale, &fresh)?;
    for name in &stale {
        remove_owned(root, review_dir, &review_dir.join(name))?;
    }
    let mut pairs = Vec::new();
    for (name, target) in fresh {
        let link = review_dir.join(name);
        if link_exists(&link)? {
            return Err(format!("{} already exists", link.display()));
        }
        pairs.push((target, link));
    }
    failures.extend(create(&pairs));
    Ok(failures)
}

// A name another folder also produces depends on the full walk's order, so only the full rebuild may settle it.
fn check_unshared(root: &Path, data: &Mapping, stale: &[String], fresh: &[(String, PathBuf)]) -> Result<(), String> {
    let mut mine: HashMap<String, usize> = HashMap::new();
    for (name, _) in fresh {
        *mine.entry(name.to_lowercase()).or_default() += 1;
    }
    let affected: BTreeSet<String> = stale.iter().map(|n| n.to_lowercase()).chain(mine.keys().cloned()).collect();
    // Every name starts with its category's first letter, so only categories sharing it can produce the same name.
    let initials: BTreeSet<char> = affected.iter().filter_map(|n| n.chars().next()).collect();
    let mut producing: HashMap<String, usize> = HashMap::new();
    for (category, node) in child_entries(data) {
        if !category.to_lowercase().chars().next().is_some_and(|c| initials.contains(&c)) {
            continue;
        }
        for owner in branch_owners(data, &[category.to_string()], node) {
            let Ok(name) = review_shortcut_name(&owner, "") else { continue };
            let name = name.to_lowercase();
            if affected.contains(&name) && rev_folder(root, &owner).is_dir() {
                *producing.entry(name).or_default() += 1;
            }
        }
    }
    for name in &affected {
        let own = mine.get(name).copied().unwrap_or(0);
        if own > 1 || producing.get(name).copied().unwrap_or(0) != own {
            return Err(format!("the shortcut {name} belongs to more than one folder"));
        }
    }
    Ok(())
}

#[cfg(windows)]
fn with_lnk(link: &Path) -> PathBuf {
    let mut path = link.as_os_str().to_owned();
    path.push(".lnk");
    PathBuf::from(path)
}

// Mirrors the full rebuild's clearing rule: on Windows any .lnk file is ours.
#[cfg(windows)]
fn remove_owned(_root: &Path, _review_dir: &Path, link: &Path) -> Result<(), String> {
    let file = with_lnk(link);
    match fs::symlink_metadata(&file) {
        Ok(meta) if meta.is_file() => fs::remove_file(&file).map_err(|e| format!("Could not remove {}: {e}", file.display())),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Could not inspect {}: {e}", file.display())),
    }
}

#[cfg(not(windows))]
fn remove_owned(root: &Path, review_dir: &Path, link: &Path) -> Result<(), String> {
    match fs::symlink_metadata(link) {
        Ok(meta) if meta.file_type().is_symlink() => {
            let target = fs::read_link(link).map_err(|e| format!("Could not read link {}: {e}", link.display()))?;
            if super::review_index::targets_storage(root, review_dir, &target) {
                fs::remove_file(link).map_err(|e| format!("Could not remove {}: {e}", link.display()))?;
            }
            Ok(())
        }
        Ok(_) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Could not inspect {}: {e}", link.display())),
    }
}

fn link_exists(link: &Path) -> Result<bool, String> {
    #[cfg(windows)]
    let link = &with_lnk(link);
    match fs::symlink_metadata(link) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("Could not inspect {}: {e}", link.display())),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::config::NewKind;
    use crate::index::review_index::rebuild_with;
    use crate::index::{reindex_paths, ReviewIndexEvent};
    use crate::mutations::{self, MoveDirection, Mutation};
    use crate::test_support::{delete_with, ok_recycle, rel, storage, yaml, TempDir};
    use crate::tree::tree_store;

    const TREE: &str = "100.ACADEMIC:\n  type: NODE-L1\n  01.Math:\n    type: NODE-L2\n    01.Algebra: {type: NODE-L3}\n  \
        02.Bio: {type: NODE-L2}\n200.HOME:\n  type: NODE-L1\n  01.Garden: {type: NODE-L2}\n";
    const DIRS: &[&str] = &[
        "998.Review",
        "100.ACADEMIC/98.ACADEMIC",
        "100.ACADEMIC/01.Math/98.Math",
        "100.ACADEMIC/01.Math/01.Algebra/98.Algebra",
        "100.ACADEMIC/02.Bio/98.Bio",
        "200.HOME/98.HOME",
        "200.HOME/01.Garden/98.Garden",
    ];

    // Stands in for PowerShell: each .lnk file holds its target path.
    #[cfg(windows)]
    fn fake_create(pairs: &[(PathBuf, PathBuf)]) -> Vec<String> {
        for (target, link) in pairs {
            fs::write(with_lnk(link), target.to_string_lossy().as_bytes()).unwrap();
        }
        Vec::new()
    }

    #[cfg(not(windows))]
    fn fake_create(pairs: &[(PathBuf, PathBuf)]) -> Vec<String> {
        crate::ant_core::shortcuts::create_shortcuts(pairs)
    }

    // A synced, saved yaml and a fully built 998.Review, as after startup.
    fn reviewed(tag: &str) -> TempDir {
        let dir = storage(tag, DIRS, TREE);
        let mut data = yaml(dir.path());
        tree_store::sync(dir.path(), &mut data).unwrap();
        tree_store::save(dir.path(), &data).unwrap();
        assert!(rebuild_with(dir.path(), &data, fake_create).is_empty());
        dir
    }

    fn links(dir: &TempDir) -> Vec<(String, PathBuf)> {
        let mut out = Vec::new();
        for entry in fs::read_dir(dir.path().join("998.Review")).unwrap() {
            let path = entry.unwrap().path();
            #[cfg(windows)]
            let target = path.is_file().then(|| PathBuf::from(fs::read_to_string(&path).unwrap()));
            #[cfg(not(windows))]
            let target = fs::read_link(&path).ok();
            if let Some(target) = target {
                out.push((path.file_name().unwrap().to_string_lossy().into_owned(), target));
            }
        }
        out.sort();
        out
    }

    fn quick(dir: &TempDir, m: &Mutation, rebuild: impl FnOnce(&Path, &Mapping) -> Vec<String>) -> (Vec<String>, usize) {
        let created = Cell::new(0);
        let create = |pairs: &[(PathBuf, PathBuf)]| {
            created.set(created.get() + pairs.len());
            fake_create(pairs)
        };
        let failures = reindex_paths(dir.path(), &|_: &ReviewIndexEvent| Ok(()), &m.before, &m.touched, create, rebuild);
        (failures, created.get())
    }

    fn assert_like_full_rebuild(dir: &TempDir, m: &Mutation, expected_created: usize) {
        assert!(m.warnings.is_empty() && !m.tree.yaml_changed && !m.touched.is_empty(), "{:?}", m.warnings);
        let (failures, created) = quick(dir, m, |_, _| panic!("the quick update should not fall back"));
        assert!(failures.is_empty(), "{failures:?}");
        assert_eq!(created, expected_created, "only the touched branch gets new shortcuts");
        let incremental = links(dir);
        assert!(rebuild_with(dir.path(), &yaml(dir.path()), fake_create).is_empty());
        assert_eq!(incremental, links(dir));
    }

    #[test]
    fn creation_adds_only_the_new_branch_shortcuts() {
        let dir = reviewed("rev-inc-add");
        let base = links(&dir).len();
        assert_eq!(base, 6);
        let m = mutations::add_node(dir.path(), &rel(&["100.ACADEMIC"]), NewKind::Node).unwrap();
        assert_like_full_rebuild(&dir, &m, 1);
        let m = mutations::add_node(dir.path(), &rel(&["100.ACADEMIC", "01.Math"]), NewKind::Node).unwrap();
        assert_like_full_rebuild(&dir, &m, 1);
        let m = mutations::add_node(dir.path(), &rel(&["200.HOME", "01.Garden"]), NewKind::Workboard).unwrap();
        assert_like_full_rebuild(&dir, &m, 0);
        let m = mutations::add_node(dir.path(), &[], NewKind::Node).unwrap();
        assert_like_full_rebuild(&dir, &m, 1);
        assert_eq!(links(&dir).len(), base + 3);
    }

    #[test]
    fn deletion_removes_only_the_deleted_branch_shortcuts() {
        let dir = reviewed("rev-inc-delete");
        let m = delete_with(dir.path(), &rel(&["100.ACADEMIC", "01.Math"]), false, true, ok_recycle).unwrap();
        assert!(m.before.iter().any(|(p, _)| *p == rel(&["100.ACADEMIC", "01.Math"])));
        assert_like_full_rebuild(&dir, &m, 0);
        let names: Vec<String> = links(&dir).into_iter().map(|(n, _)| n).collect();
        assert_eq!(names.len(), 4, "{names:?}");
        assert!(!names.iter().any(|n| n.starts_with("101.")), "{names:?}");
    }

    #[test]
    fn renames_and_moves_recreate_only_the_changed_shortcuts() {
        let dir = reviewed("rev-inc-rename");
        let m = mutations::rename_node(dir.path(), &rel(&["100.ACADEMIC", "01.Math"]), "Physics").unwrap();
        assert_like_full_rebuild(&dir, &m, 2);
        let m = mutations::rename_node(dir.path(), &rel(&["200.HOME"]), "house").unwrap();
        assert_like_full_rebuild(&dir, &m, 2);
        let m = mutations::move_node(dir.path(), &rel(&["100.ACADEMIC", "01.Physics"]), MoveDirection::Down).unwrap();
        assert_eq!(m.select, Some(rel(&["100.ACADEMIC", "02.Physics"])));
        assert_like_full_rebuild(&dir, &m, 3);
    }

    #[test]
    fn a_mismatch_falls_back_to_the_full_rebuild_and_says_so() {
        let dir = reviewed("rev-inc-fallback");
        let review = dir.path().join("998.Review");
        // A stale link already sits where the new node's shortcut goes.
        fake_create(&[(dir.path().join("100.ACADEMIC/02.Bio/98.Bio"), review.join("103.New-Node"))]);
        let m = mutations::add_node(dir.path(), &rel(&["100.ACADEMIC"]), NewKind::Node).unwrap();
        assert_eq!(m.select, Some(rel(&["100.ACADEMIC", "03.New-Node"])));
        let rebuilt = Cell::new(false);
        let (failures, _) = quick(&dir, &m, |root, data| {
            rebuilt.set(true);
            rebuild_with(root, data, fake_create)
        });
        assert!(rebuilt.get());
        assert_eq!(failures.len(), 1, "{failures:?}");
        assert!(failures[0].contains("rebuilt in full") && failures[0].contains("already exists"), "{failures:?}");
        let fallback = links(&dir);
        assert!(fallback.iter().any(|(_, t)| t.ends_with("03.New-Node/98.New-Node")), "{fallback:?}");
        assert!(rebuild_with(dir.path(), &yaml(dir.path()), fake_create).is_empty());
        assert_eq!(fallback, links(&dir));
    }

    #[test]
    fn a_shortcut_name_shared_by_two_folders_is_a_mismatch() {
        let dir = reviewed("rev-inc-shared");
        dir.mkdir("150.SPORT/01.Garden/98.Garden");
        let mut data = yaml(dir.path());
        tree_store::sync(dir.path(), &mut data).unwrap();
        tree_store::save(dir.path(), &data).unwrap();
        let review = dir.path().join("998.Review");
        let m = mutations::rename_node(dir.path(), &rel(&["200.HOME", "01.Garden"]), "Yard").unwrap();
        let other_letter = update_with(dir.path(), &review, &yaml(dir.path()), &m.before, &m.touched, fake_create);
        assert!(other_letter.is_ok(), "{other_letter:?}");
        let m = mutations::rename_node(dir.path(), &rel(&["100.ACADEMIC", "01.Math"]), "Garden").unwrap();
        let shared = update_with(dir.path(), &review, &yaml(dir.path()), &m.before, &m.touched, |_| panic!("not created"));
        assert!(shared.is_err_and(|e| e.contains("101.garden") && e.contains("more than one folder")));
    }
}
