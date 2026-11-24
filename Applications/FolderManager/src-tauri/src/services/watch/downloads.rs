// Downloads and the other native folders live outside the storage, so each has its own watcher telling the window when to read it again.
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use notify::{Event, EventKind, RecursiveMode, Watcher};

// A download or a copy fires many events; one quiet stretch ends the batch.
const QUIET: Duration = Duration::from_millis(400);

pub fn watch_system_folder(dir: PathBuf, on_changed: impl Fn() + Send + 'static) {
    thread::spawn(move || {
        if let Err(e) = run(&dir, &on_changed) {
            super::log("error", &format!("Watch of {} stopped: {e}", dir.display()));
        }
    });
}

fn run(dir: &Path, on_changed: &dyn Fn()) -> notify::Result<()> {
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(dir, RecursiveMode::Recursive)?;
    let mut pending = false;
    loop {
        match rx.recv_timeout(QUIET) {
            Ok(Ok(event)) if matches!(event.kind, EventKind::Access(_)) => {}
            Ok(_) => pending = true,
            Err(RecvTimeoutError::Timeout) if pending => {
                pending = false;
                on_changed();
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
        }
    }
}
