// Sends a running copy's progress to the window a few times a second and its end once; the window can pause or cancel it.
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter};

use crate::services::transfer::{self, Job};

const EVENT: &str = "fm://transfer";
const DONE_EVENT: &str = "fm://transfer-done";
const TICK: Duration = Duration::from_millis(250);

pub(crate) struct Tracked {
    app: AppHandle,
    stop: Arc<AtomicBool>,
    _job: Job,
}

pub(crate) fn track(app: &AppHandle, sources: &[PathBuf], to: &str) -> Tracked {
    let job = transfer::begin(sources, to);
    let stop = Arc::new(AtomicBool::new(false));
    let (done, sender) = (stop.clone(), app.clone());
    std::thread::spawn(move || {
        while !done.load(Ordering::Relaxed) {
            if let Some(p) = transfer::snapshot() {
                if let Err(e) = sender.emit(EVENT, &p) {
                    eprintln!("Could not send {EVENT}: {e}");
                }
            }
            std::thread::sleep(TICK);
        }
    });
    Tracked { app: app.clone(), stop, _job: job }
}

impl Drop for Tracked {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Err(e) = self.app.emit(DONE_EVENT, ()) {
            eprintln!("Could not send {DONE_EVENT}: {e}");
        }
    }
}

#[tauri::command]
pub fn cancel_transfer() {
    transfer::cancel();
}

#[tauri::command]
pub fn pause_transfer(paused: bool) {
    transfer::set_paused(paused);
}
