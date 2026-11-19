// Shows a running copy in its own window once it outlasts a moment, sends it progress a few times a second, and closes it at the end.
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use super::off_thread;
use crate::domain::error::R;
use crate::app::storage_session::Storage;

use crate::services::transfer::{self, Job};

const EVENT: &str = "fm://transfer";
const TICK: Duration = Duration::from_millis(250);
const SHOW_AFTER: Duration = Duration::from_millis(500);
const LABEL: &str = "transfer";

fn open_window(app: &AppHandle) {
    if app.get_webview_window(LABEL).is_some() {
        return;
    }
    let built = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title("Transfer")
        .inner_size(760.0, 170.0)
        .min_inner_size(520.0, 150.0)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .theme(Some(tauri::Theme::Dark))
        .additional_browser_args(super::dictionary::BROWSER_ARGS)
        .initialization_script("window.__TRANSFER__ = true;")
        .build();
    if let Err(e) = built {
        eprintln!("Could not open the transfer window: {e}");
    }
}

pub(crate) struct Tracked {
    app: AppHandle,
    stop: Arc<AtomicBool>,
    _job: Job,
}

pub(crate) fn track(app: &AppHandle, sources: &[PathBuf], to: &str) -> Tracked {
    watch(app, transfer::begin(sources, to))
}

fn watch(app: &AppHandle, job: Job) -> Tracked {
    let stop = Arc::new(AtomicBool::new(false));
    let (done, sender) = (stop.clone(), app.clone());
    std::thread::spawn(move || {
        let started = Instant::now();
        while !done.load(Ordering::Relaxed) {
            if started.elapsed() >= SHOW_AFTER {
                open_window(&sender);
            }
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
        if let Some(w) = self.app.get_webview_window(LABEL) {
            if let Err(e) = w.destroy() {
                eprintln!("Could not close the transfer window: {e}");
            }
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

// A deletion is one call per item from the window, so the window opens the job around them and closes it after.
static DELETING: Mutex<Option<Tracked>> = Mutex::new(None);

#[tauri::command]
pub async fn begin_delete(app: AppHandle, targets: Vec<Vec<String>>) -> R<()> {
    let sender = app.clone();
    off_thread(move || {
        let root = app.state::<Storage>().root().ok();
        let downloads = super::downloads::base(&app).ok();
        let paths: Vec<PathBuf> = targets
            .iter()
            .filter_map(|rel| match super::downloads::below_downloads(rel) {
                Some(rest) => downloads.as_ref().and_then(|b| crate::services::downloads::path_of(b, rest).ok()),
                None => root.as_ref().and_then(|r| crate::services::mutations::source_path(r, rel).ok()),
            })
            .collect();
        let job = watch(&sender, transfer::begin_as("Deleting", &paths, "Recycle Bin", true));
        *DELETING.lock().unwrap_or_else(|e| e.into_inner()) = Some(job);
        Ok(())
    })
    .await?
}

#[tauri::command]
pub fn end_delete() {
    DELETING.lock().unwrap_or_else(|e| e.into_inner()).take();
}

// Each deleted item reports itself; outside a deletion job this does nothing.
pub(crate) fn recycle_tracked(path: &std::path::Path, recycle: impl Fn(&std::path::Path) -> Result<(), String>) -> Result<(), String> {
    transfer::checkpoint().map_err(|e| e.to_string())?;
    let bytes = transfer::size_of(path);
    recycle(path)?;
    transfer::item_done(&path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), bytes);
    Ok(())
}
