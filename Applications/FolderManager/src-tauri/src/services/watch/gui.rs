// The window's watcher: one thread on the open storage, moved to the new root when the root changes.
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde::Serialize;

use super::{log, watch, ApplyReport};
use crate::infra::storage::changes::{self, News};

// A rename or location change only swaps the root in memory, so polling it is how the watcher notices.
const FOLLOW: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StorageChanged {
    pub paths: usize,
    pub folders: Vec<Vec<String>>,
    pub rescan: bool,
    pub dictionary: bool,
}

// Our own writes are filtered out before this point, so any remaining path is news; files alone save no tree.
fn changed_event(report: &ApplyReport, news: Option<News>) -> Option<StorageChanged> {
    let Some(news) = news else {
        return (report.saved || report.paths > 0 || report.dictionary).then(|| StorageChanged { paths: report.paths, folders: report.folders.clone(), rescan: report.full, dictionary: report.dictionary });
    };
    let mut folders = report.folders.clone();
    folders.extend(news.levels);
    folders.sort();
    folders.dedup();
    Some(StorageChanged { paths: report.paths, folders, rescan: report.full || news.rescan, dictionary: report.dictionary })
}

// Read on every batch rather than on its own event, so a log write that landed in an earlier batch is still found.
fn news_since(root: &Path, seen: &mut u64) -> Option<News> {
    match changes::read(root) {
        Ok(entries) => {
            let news = changes::news(&entries, *seen, std::process::id());
            *seen = changes::newest(&entries);
            news
        }
        Err(e) => {
            log("warning", &format!("Watch: change log not read: {e}"));
            None
        }
    }
}

struct Running {
    root: PathBuf,
    stop: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

#[derive(Default)]
struct Follower {
    running: Option<Running>,
}

type Spawn<'a> = &'a dyn Fn(PathBuf, Arc<AtomicBool>) -> JoinHandle<()>;

impl Follower {
    fn follow(&mut self, wanted: Option<PathBuf>, spawn: Spawn) {
        if self.running.as_ref().map(|r| &r.root) == wanted.as_ref() {
            return;
        }
        self.stop();
        if let Some(root) = wanted {
            let stop = Arc::new(AtomicBool::new(false));
            let handle = spawn(root.clone(), stop.clone());
            self.running = Some(Running { root, stop, handle });
        }
    }

    fn stop(&mut self) {
        if let Some(running) = self.running.take() {
            running.stop.store(true, Ordering::Relaxed);
            if running.handle.join().is_err() {
                log("error", &format!("Watch: the watcher of {} panicked", running.root.display()));
            }
        }
    }
}

fn run(root: &Path, stop: &AtomicBool, on_changed: &dyn Fn(StorageChanged)) {
    log("info", &format!("Watch: window watcher started on {}", root.display()));
    // Changes made before the window opened are already in what it loaded.
    let mut seen = 0;
    news_since(root, &mut seen);
    let mut on_applied = |report: &ApplyReport| {
        if let Some(event) = changed_event(report, news_since(root, &mut seen)) {
            on_changed(event);
        }
    };
    if let Err(e) = watch(root, stop, &mut on_applied) {
        log("error", &format!("Watch: window watcher stopped: {e}"));
    }
}

pub struct GuiWatch {
    exit: Arc<AtomicBool>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl GuiWatch {
    pub fn start<F, C>(root_of: F, on_changed: C) -> Self
    where
        F: Fn() -> Option<PathBuf> + Send + 'static,
        C: Fn(StorageChanged) + Send + Sync + 'static,
    {
        let exit = Arc::new(AtomicBool::new(false));
        let flag = exit.clone();
        let on_changed = Arc::new(on_changed);
        let handle = thread::spawn(move || {
            let mut follower = Follower::default();
            let spawn = |root: PathBuf, stop: Arc<AtomicBool>| {
                let on_changed = on_changed.clone();
                thread::spawn(move || run(&root, &stop, &*on_changed))
            };
            while !flag.load(Ordering::Relaxed) {
                follower.follow(root_of(), &spawn);
                thread::sleep(FOLLOW);
            }
            follower.stop();
        });
        Self { exit, handle: Mutex::new(Some(handle)) }
    }

    // Waiting for the thread lets a save that is already running finish before the process ends.
    pub fn shutdown(&self) {
        self.exit.store(true, Ordering::Relaxed);
        let handle = self.handle.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take();
        if let Some(handle) = handle {
            if handle.join().is_err() {
                log("error", "Watch: the window watcher supervisor panicked");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn only_a_batch_that_changed_something_reaches_the_window() {
        let quiet = ApplyReport { paths: 0, full: false, skipped: 3, saved: false, folders: vec![], dictionary: false };
        assert_eq!(changed_event(&quiet, None), None);
        let folders = vec![vec!["100.WORK".to_string()]];
        let files = ApplyReport { paths: 2, full: false, skipped: 1, saved: false, folders: folders.clone(), dictionary: false };
        assert_eq!(changed_event(&files, None), Some(StorageChanged { paths: 2, folders, rescan: false, dictionary: false }));
        let saved = ApplyReport { saved: true, full: true, folders: vec![], ..files };
        assert_eq!(changed_event(&saved, None), Some(StorageChanged { paths: 2, folders: vec![], rescan: true, dictionary: false }));
    }

    #[test]
    fn a_metadata_change_from_another_process_reaches_the_window_merged_with_the_batch() {
        let quiet = ApplyReport { paths: 0, full: false, skipped: 1, saved: false, folders: vec![], dictionary: false };
        let level = |p: &str| vec![p.to_string()];
        let news = News { levels: vec![level("200.HOME"), level("100.WORK")], rescan: false };
        assert_eq!(changed_event(&quiet, Some(news)), Some(StorageChanged { paths: 0, folders: vec![level("100.WORK"), level("200.HOME")], rescan: false, dictionary: false }));
        let disk = ApplyReport { paths: 1, folders: vec![level("100.WORK")], ..quiet };
        let rescan = News { levels: vec![level("100.WORK")], rescan: true };
        assert_eq!(changed_event(&disk, Some(rescan)), Some(StorageChanged { paths: 1, folders: vec![level("100.WORK")], rescan: true, dictionary: false }));
    }

    #[test]
    fn the_log_is_read_from_where_the_window_last_stopped() {
        let dir = crate::test_support::TempDir::new("watch-news");
        let mut seen = 0;
        assert_eq!(news_since(dir.path(), &mut seen), None);
        changes::record(dir.path(), &[vec!["100.WORK".to_string()]], false).unwrap();
        assert_eq!(news_since(dir.path(), &mut seen), None);
        assert_eq!(seen, 1);
        let log = std::fs::read_to_string(dir.path().join(".ant").join(changes::CHANGES_FILE)).unwrap();
        let foreign = log.replace(&format!("\"writer\":{}", std::process::id()), "\"writer\":0").replace("\"seq\":1", "\"seq\":2");
        std::fs::write(dir.path().join(".ant").join(changes::CHANGES_FILE), foreign).unwrap();
        assert_eq!(news_since(dir.path(), &mut seen), Some(News { levels: vec![vec!["100.WORK".to_string()]], rescan: false }));
        assert_eq!(news_since(dir.path(), &mut seen), None);
    }

    #[test]
    fn a_new_root_stops_the_old_watcher_before_starting_one() {
        let flags = RefCell::new(Vec::<(PathBuf, Arc<AtomicBool>)>::new());
        let spawn = |root: PathBuf, stop: Arc<AtomicBool>| {
            flags.borrow_mut().push((root, stop.clone()));
            thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(5));
                }
            })
        };
        let mut follower = Follower::default();
        follower.follow(Some(PathBuf::from("A")), &spawn);
        follower.follow(Some(PathBuf::from("A")), &spawn);
        assert_eq!(flags.borrow().len(), 1);
        follower.follow(Some(PathBuf::from("B")), &spawn);
        let started: Vec<(PathBuf, bool)> = flags.borrow().iter().map(|(r, s)| (r.clone(), s.load(Ordering::Relaxed))).collect();
        assert_eq!(started, vec![(PathBuf::from("A"), true), (PathBuf::from("B"), false)]);
        follower.follow(None, &spawn);
        assert!(flags.borrow()[1].1.load(Ordering::Relaxed));
        assert!(follower.running.is_none());
    }
}
