// Recursive notify watcher that gathers events into batches separated by a quiet period.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecursiveMode, Watcher};

use super::{apply, apply_batch, log, ApplyReport};
use crate::error::{FmError, R};

const QUIET: Duration = Duration::from_secs(2);
const POLL: Duration = Duration::from_millis(200);
// Past this many paths the batch is a full sync anyway, so more paths would only cost memory.
const MAX_PENDING: usize = 10_000;

#[derive(Default)]
struct Batch {
    paths: BTreeSet<PathBuf>,
    rescan: Option<String>,
    last: Option<Instant>,
}

impl Batch {
    fn add(&mut self, event: notify::Result<Event>) {
        match event {
            Ok(event) if matches!(event.kind, EventKind::Access(_)) => return,
            Ok(event) => {
                if event.need_rescan() {
                    self.rescan.get_or_insert_with(|| "the watcher asked for a rescan".to_string());
                }
                for path in event.paths {
                    if self.paths.len() < MAX_PENDING {
                        self.paths.insert(path);
                    } else {
                        self.rescan.get_or_insert_with(|| format!("more than {MAX_PENDING} changed paths"));
                    }
                }
            }
            Err(e) => {
                self.rescan.get_or_insert_with(|| format!("watcher error: {e}"));
            }
        }
        self.last = Some(Instant::now());
    }

    fn settled(&self) -> bool {
        self.last.is_some_and(|t| t.elapsed() >= QUIET)
    }
}

pub fn watch(root: &Path, stop: &AtomicBool, on_applied: &mut dyn FnMut(&ApplyReport)) -> R<()> {
    let (tx, rx) = mpsc::channel();
    let start_err = |e: notify::Error| FmError::Internal { message: format!("Could not watch {}: {e}", root.display()) };
    let mut watcher = notify::recommended_watcher(tx).map_err(start_err)?;
    watcher.watch(root, RecursiveMode::Recursive).map_err(start_err)?;
    let mut batch = Batch::default();
    while !stop.load(Ordering::Relaxed) {
        match rx.recv_timeout(POLL) {
            Ok(event) => batch.add(event),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err(FmError::Internal { message: format!("The watcher of {} stopped", root.display()) });
            }
        }
        if batch.settled() {
            let done = std::mem::take(&mut batch);
            let paths: Vec<PathBuf> = done.paths.into_iter().collect();
            let applied = match &done.rescan {
                Some(reason) => apply::apply(root, &paths, Some(reason)),
                None => apply_batch(root, &paths),
            };
            match applied {
                Ok(report) => on_applied(&report),
                Err(e) => log("error", &format!("Watch: batch of {} paths not applied: {e}", paths.len())),
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{rel, work, yaml};
    use crate::tree::tree_store;

    #[test]
    fn an_error_or_too_many_paths_turn_the_batch_into_a_rescan() {
        let mut batch = Batch::default();
        batch.add(Ok(Event::new(EventKind::Any).add_path(PathBuf::from("a"))));
        assert!(batch.rescan.is_none() && !batch.settled());
        batch.add(Err(notify::Error::generic("boom")));
        assert!(batch.rescan.as_deref().is_some_and(|r| r.contains("boom")));
        let mut full = Batch::default();
        let many = (0..=MAX_PENDING).fold(Event::new(EventKind::Any), |e, n| e.add_path(PathBuf::from(n.to_string())));
        full.add(Ok(many));
        assert_eq!(full.paths.len(), MAX_PENDING);
        assert!(full.rescan.is_some());
    }

    #[test]
    fn a_folder_made_while_watching_is_applied_then_the_watch_stops() {
        let dir = work("watch-live");
        let root = dir.path().to_path_buf();
        let stop = std::sync::Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        let flag = stop.clone();
        let handle = std::thread::spawn(move || watch(&root, &flag, &mut |r| tx.send(r.clone()).unwrap()));
        std::thread::sleep(Duration::from_millis(500));
        dir.mkdir("100.WORK/06.New");
        let report = rx.recv_timeout(Duration::from_secs(20)).unwrap();
        stop.store(true, Ordering::Relaxed);
        handle.join().unwrap().unwrap();
        assert!(report.paths >= 1, "{report:?}");
        assert_eq!(tree_store::get_type(&yaml(dir.path()), &rel(&["100.WORK", "06.New"])), "NODE-L2");
    }

    // notify opens the root with FILE_SHARE_DELETE, so the storage can still be renamed while it is watched.
    #[cfg(windows)]
    #[test]
    fn the_watched_storage_can_be_renamed_and_the_watch_still_stops() {
        let parent = crate::test_support::TempDir::new("watch-rename");
        parent.mkdir("Store/999.Settings/00.Core");
        parent.mkdir("Store/100.WORK");
        let (root, renamed) = (parent.path().join("Store"), parent.path().join("Renamed"));
        tree_store::save(&root, &serde_yaml::from_str("100.WORK: {type: NODE-L1}").unwrap()).unwrap();
        let stop = std::sync::Arc::new(AtomicBool::new(false));
        let (flag, watched) = (stop.clone(), root.clone());
        let handle = std::thread::spawn(move || watch(&watched, &flag, &mut |_| {}));
        std::thread::sleep(Duration::from_millis(500));
        std::fs::rename(&root, &renamed).unwrap();
        std::thread::sleep(QUIET + Duration::from_millis(500));
        stop.store(true, Ordering::Relaxed);
        handle.join().unwrap().unwrap();
        assert!(!root.exists());
        assert!(renamed.join(".ant/ant.db").is_file());
    }
}
