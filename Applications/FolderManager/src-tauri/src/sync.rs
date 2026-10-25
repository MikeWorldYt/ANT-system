// Daily headless refresh that keeps the stored tree in step with folders changed outside the app.
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::error::R;
use crate::index::review_index;
use crate::scheduler::{self, Task};
use crate::tree::{snapshot, tree_store};

const TASK: Task = Task { name: "ANT System Manager Sync", flag: "--sync", start: "12:30", what: "sync" };

pub fn apply(on: bool, root: &Path) -> R<()> {
    scheduler::apply(&TASK, on.then_some("DAILY"), root)
}

fn sync(root: &Path) -> R<()> {
    snapshot(root)?;
    for failure in review_index::rebuild(root, &tree_store::load(root)?) {
        eprintln!("Review index: {failure}");
    }
    Ok(())
}

// A storage that was moved or deleted is left alone; its task is stale, not broken.
pub fn run_headless(start: Option<OsString>) -> i32 {
    let Some(path) = start.map(PathBuf::from) else {
        eprintln!("--sync needs the path of an ANT storage");
        return 1;
    };
    if !path.is_dir() {
        return 0;
    }
    let Some(root) = crate::ant_core::names::find_root(&path) else {
        eprintln!("{} is not an ANT storage", path.display());
        return 1;
    };
    match sync(&root) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Sync failed: {e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{rel, work, TempDir};

    #[test]
    fn schtasks_arguments_run_the_sync_daily() {
        let args = scheduler::args(&TASK, Some("DAILY"), Path::new("C:/App/fm.exe"), Path::new("D:/My Storage"));
        let expected = ["/Create", "/F", "/SC", "DAILY", "/TN", "ANT System Manager Sync", "/TR"];
        assert_eq!(args[..7], expected);
        assert_eq!(args[7], "\"C:/App/fm.exe\" --sync \"D:/My Storage\"");
        assert_eq!(args[8..], ["/ST", "12:30"]);
        let off = scheduler::args(&TASK, None, Path::new("a"), Path::new("b"));
        assert_eq!(off, ["/Delete", "/F", "/TN", "ANT System Manager Sync"]);
    }

    #[test]
    fn a_missing_storage_is_skipped_and_an_existing_one_is_synced() {
        let gone = TempDir::new("sync-gone");
        assert_eq!(run_headless(Some(gone.path().join("Moved").into())), 0);
        assert_eq!(run_headless(Some(gone.path().into())), 1);
        assert_eq!(run_headless(None), 1);
        let dir = work("sync-run");
        dir.mkdir("100.WORK/06.New");
        assert_eq!(run_headless(Some(dir.path().into())), 0);
        let data = tree_store::load(dir.path()).unwrap();
        assert!(tree_store::get_node(&data, &rel(&["100.WORK", "06.New"])).is_some());
    }
}
