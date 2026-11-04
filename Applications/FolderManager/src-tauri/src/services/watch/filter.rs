// Decides which changed paths matter: our own writes are dropped so a sync never triggers the next one.
use std::path::{Component, Path};

use crate::config::{is_hidden_dir, DB_FILENAME, REVIEW_FOLDER};
use crate::fs_safe::LOCK_FILE;

pub(super) const FULL_SYNC_AFTER: usize = 200;
const TREE_MIRROR: &str = "Tree.yaml";

pub(super) fn needs_full(distinct: usize) -> bool {
    distinct > FULL_SYNC_AFTER
}

// None for a path outside the storage, as the watcher may report the root's own parent moves.
pub(super) fn relative(root: &Path, path: &Path) -> Option<Vec<String>> {
    let rest = path.strip_prefix(root).ok()?;
    rest.components()
        .map(|c| match c {
            Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect()
}

// ant.db also covers its -journal, -wal and -shm files, which SQLite writes next to it.
pub(super) fn is_own_write(rel: &[String]) -> bool {
    let in_review = rel.first().is_some_and(|first| first.eq_ignore_ascii_case(REVIEW_FOLDER));
    let own_file = rel.last().is_some_and(|name| {
        name.eq_ignore_ascii_case(TREE_MIRROR)
            || name.eq_ignore_ascii_case(LOCK_FILE)
            || name.to_ascii_lowercase().starts_with(DB_FILENAME)
    });
    in_review || own_file || rel.iter().any(|part| is_hidden_dir(part))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::rel;

    #[test]
    fn our_own_writes_are_ignored_and_a_normal_folder_is_not() {
        for parts in [
            &[".ant", "ant.db"][..],
            &[".ant", "debug", "26-10-29-10-00_ANT-fm.log"],
            &[".ANT"],
            &["ant.db-journal"],
            &[".ant", "ant.db-wal"],
            &["100.WORK", "Tree.yaml"],
            &["998.Review"],
            &["998.review", "01.Math.lnk"],
            &["999.Settings", "00.Core", ".FolderManager.lock"],
            &[".ant-trash", "x"],
        ] {
            assert!(is_own_write(&rel(parts)), "{parts:?}");
        }
        for parts in [&["100.WORK", "06.New"][..], &["100.WORK", "998.Review"], &["999.Settings", "00.Core"], &[]] {
            assert!(!is_own_write(&rel(parts)), "{parts:?}");
        }
    }

    #[test]
    fn paths_are_made_relative_and_outside_ones_dropped() {
        let root = Path::new("/s/My Storage");
        assert_eq!(relative(root, &root.join("100.WORK/06.New")), Some(rel(&["100.WORK", "06.New"])));
        assert_eq!(relative(root, root), Some(Vec::new()));
        assert_eq!(relative(root, Path::new("/s/Other/100.WORK")), None);
        assert_eq!(relative(root, &root.join("../x")), None);
    }

    #[test]
    fn more_than_two_hundred_paths_need_a_full_sync() {
        assert!(!needs_full(0));
        assert!(!needs_full(200));
        assert!(needs_full(201));
    }
}
