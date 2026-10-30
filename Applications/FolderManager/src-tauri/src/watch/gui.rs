// The window's watcher: one thread on the open storage, moved to the new root when the root changes.
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde::Serialize;

use super::{log, watch, ApplyReport};

// A rename or location change only swaps the root in memory, so polling it is how the watcher notices.
const FOLLOW: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct StorageChanged {
    pub paths: usize,
    pub full: bool,
}

// Our own mutations wake the watcher too; only a batch that saved the tree is news to the window.
fn changed_event(report: &ApplyReport) -> Option<StorageChanged> {
    report.saved.then_some(StorageChanged { paths: report.paths, full: report.full })
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
    let mut on_applied = |report: &ApplyReport| {
        if let Some(event) = changed_event(report) {
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
    fn only_a_saved_batch_reaches_the_window() {
        let report = ApplyReport { paths: 2, full: false, skipped: 1, saved: false };
        assert_eq!(changed_event(&report), None);
        let saved = ApplyReport { saved: true, full: true, ..report };
        assert_eq!(changed_event(&saved), Some(StorageChanged { paths: 2, full: true }));
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
