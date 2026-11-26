// Opens the transfer window before a copy, move or deletion starts, sends it progress a few times a second, and closes it a moment after the end.
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder};

use super::off_thread;
use crate::domain::error::R;
use crate::app::storage_session::Storage;

use crate::services::transfer::{self, Job};

const EVENT: &str = "fm://transfer";
const TICK: Duration = Duration::from_millis(250);
const READY_WAIT: Duration = Duration::from_secs(2);
const LINGER: Duration = Duration::from_secs(3);
const READY_EVENT: &str = "fm://transfer-ready";
const LABEL: &str = "transfer";
const MARGIN: f64 = 12.0;

// Waits until the new window listens, so its first frame already has the job; an open window is reused at once.
fn open_window(app: &AppHandle) {
    if app.get_webview_window(LABEL).is_some() {
        return;
    }
    let (tx, rx) = std::sync::mpsc::channel();
    let id = app.once(READY_EVENT, move |_| {
        // The wait may have timed out already; nobody listens then.
        let _ = tx.send(());
    });
    let built = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title("Transfer")
        .inner_size(840.0, 236.0)
        .min_inner_size(640.0, 220.0)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .background_color(tauri::window::Color(0, 0, 0, 0))
        .always_on_top(true)
        .theme(Some(tauri::Theme::Dark))
        .additional_browser_args(super::dictionary::BROWSER_ARGS)
        .initialization_script("window.__TRANSFER__ = true;")
        .visible(false)
        .build();
    match built {
        Ok(w) => {
            place_in_corner(app, &w);
            if let Err(e) = w.show() {
                eprintln!("Could not show the transfer window: {e}");
            }
        }
        Err(e) => eprintln!("Could not open the transfer window: {e}"),
    }
    if rx.recv_timeout(READY_WAIT).is_err() {
        app.unlisten(id);
    }
}

// The bottom right corner of the screen the app is on, above the taskbar, so it never lands over the work.
fn place_in_corner(app: &AppHandle, w: &tauri::WebviewWindow) {
    let monitor = app.get_webview_window("main").and_then(|m| m.current_monitor().ok().flatten()).or_else(|| w.primary_monitor().ok().flatten());
    let (Some(monitor), Ok(size)) = (monitor, w.outer_size()) else { return };
    let area = monitor.work_area();
    let margin = (MARGIN * monitor.scale_factor()) as i32;
    let x = area.position.x + area.size.width as i32 - size.width as i32 - margin;
    let y = area.position.y + area.size.height as i32 - size.height as i32 - margin;
    if let Err(e) = w.set_position(tauri::PhysicalPosition::new(x, y)) {
        eprintln!("Could not place the transfer window: {e}");
    }
}

fn close_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(LABEL) {
        if let Err(e) = w.destroy() {
            eprintln!("Could not close the transfer window: {e}");
        }
    }
}

pub(crate) struct Tracked {
    app: AppHandle,
    stop: Arc<AtomicBool>,
    _job: Job,
}

pub(crate) fn track(app: &AppHandle, sources: &[PathBuf], to: &str, copy: bool) -> Tracked {
    watch(app, transfer::begin_as(if copy { "Copying" } else { "Moving" }, sources, to, false))
}

fn watch(app: &AppHandle, job: Job) -> Tracked {
    open_window(app);
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
        // The final numbers stay in sight a moment before the window goes.
        std::thread::sleep(LINGER);
        close_window(&sender);
    });
    Tracked { app: app.clone(), stop, _job: job }
}

// The last numbers are sent while the job still exists, so even an instant move shows what it did.
impl Drop for Tracked {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(mut p) = transfer::snapshot() {
            // A move within one drive renames instead of copying, so nothing was counted; it is done whole.
            if p.bytes_done == 0 && p.files_done == 0 {
                (p.bytes_done, p.files_done) = (p.bytes_total, p.files_total);
            }
            p.finished = true;
            if let Err(e) = self.app.emit(EVENT, &p) {
                eprintln!("Could not send {EVENT}: {e}");
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
        let paths: Vec<PathBuf> = targets
            .iter()
            .filter_map(|rel| match super::downloads::system_path(&app, rel) {
                Some(path) => path.ok(),
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
