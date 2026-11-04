// The `--watch <root>` mode: one long-lived watcher per storage, started without the GUI.
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;

use super::{guard, log, tray, watch};
use crate::backup::stamp::local_stamp;
use crate::debug_log::{self, sysinfo};

// A storage that was moved or deleted is left alone; its logon task is stale, not broken.
pub fn run_headless(start: Option<OsString>) -> i32 {
    let Some(path) = start.map(PathBuf::from) else {
        eprintln!("--watch needs the path of an ANT storage");
        return 1;
    };
    if !path.is_dir() {
        return 0;
    }
    let Some(root) = crate::ant_core::names::find_root(&path) else {
        eprintln!("{} is not an ANT storage", path.display());
        return 1;
    };
    let _guard = match guard::guard_dir().and_then(|dir| guard::acquire(&dir, &root)) {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Watch not started: {e}");
            return 1;
        }
    };
    if let Err(e) = debug_log::start(&local_stamp(), &root, &sysinfo::collect()) {
        eprintln!("Debug log not started: {e}");
    }
    log("info", &format!("Watch: started on {}", root.display()));
    let stop = Arc::new(AtomicBool::new(false));
    // The tray must pump messages on the thread that made it, so the watch gets its own thread.
    let watcher = {
        let (root, stop) = (root.clone(), stop.clone());
        thread::spawn(move || {
            let mut on_applied = |r: &super::ApplyReport| {
                log("info", &format!("Watch: applied {} paths, {} skipped, full sync: {}", r.paths, r.skipped, r.full));
            };
            watch(&root, &stop, &mut on_applied)
        })
    };
    tray::show(&root, &stop, &watcher);
    match watcher.join() {
        Ok(Ok(())) => 0,
        Ok(Err(e)) => {
            log("error", &format!("Watch: stopped: {e}"));
            eprintln!("Watch failed: {e}");
            1
        }
        Err(_) => {
            log("error", "Watch: the watcher thread panicked");
            eprintln!("Watch failed: the watcher thread panicked");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{work, TempDir};

    #[test]
    fn a_second_watcher_on_the_same_storage_fails_fast() {
        let dir = work("watch-guard");
        let guards = guard::guard_dir().unwrap();
        let held = guard::acquire(&guards, dir.path()).unwrap();
        let started = std::time::Instant::now();
        assert_eq!(run_headless(Some(dir.path().into())), 1);
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        drop(held);
        std::fs::remove_file(guards.join(guard::guard_name(dir.path()))).unwrap();
    }

    #[test]
    fn a_missing_or_unknown_storage_is_not_watched() {
        let gone = TempDir::new("watch-gone");
        assert_eq!(run_headless(Some(gone.path().join("Moved").into())), 0);
        assert_eq!(run_headless(Some(gone.path().into())), 1);
        assert_eq!(run_headless(None), 1);
    }
}
