// Notification-area icon of the headless watcher, so a background watch is visible and can be stopped.
// The lib test exe has no common-controls v6 manifest and fails to load once muda is linked, so tests skip the tray.
#![cfg_attr(any(not(windows), test), allow(dead_code))]
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::thread::JoinHandle;

use super::log;

pub const OPEN_LABEL: &str = "Open FolderManager";
pub const STOP_LABEL: &str = "Stop watching";
// Windows keeps 128 UTF-16 units for the tooltip and tray-icon drops the terminator at the full length.
const TIP_UNITS: usize = 127;

#[derive(Debug, PartialEq, Eq)]
pub enum TrayStatus {
    Shown,
    Unavailable(String),
    #[cfg(any(not(windows), test))]
    Unsupported,
}

#[cfg_attr(any(not(windows), test), allow(unused_variables))]
pub fn show<T>(root: &Path, stop: &AtomicBool, watcher: &JoinHandle<T>) -> TrayStatus {
    #[cfg(all(windows, not(test)))]
    return status(win::run(root, stop, watcher));
    #[cfg(any(not(windows), test))]
    {
        log("info", "Watch: no tray icon on this platform");
        TrayStatus::Unsupported
    }
}

// A missing tray only costs the icon; the watcher keeps running either way.
fn status(shown: Result<(), String>) -> TrayStatus {
    match shown {
        Ok(()) => TrayStatus::Shown,
        Err(e) => {
            log("error", &format!("Watch: running without a tray icon: {e}"));
            TrayStatus::Unavailable(e)
        }
    }
}

pub fn storage_name(root: &Path) -> String {
    root.file_name().map_or_else(|| root.display().to_string(), |n| n.to_string_lossy().into_owned())
}

pub fn tooltip(root: &Path) -> String {
    fit(&format!("FolderManager: watching {}", storage_name(root)), TIP_UNITS)
}

// Menus read `&` as a mnemonic marker, so it is doubled to show the path as typed.
pub fn path_label(root: &Path) -> String {
    root.display().to_string().replace('&', "&&")
}

fn fit(text: &str, units: usize) -> String {
    if text.encode_utf16().count() <= units {
        return text.to_string();
    }
    let mut out = String::new();
    let mut used = 3;
    for c in text.chars() {
        used += c.len_utf16();
        if used > units {
            break;
        }
        out.push(c);
    }
    out + "..."
}

#[cfg(all(windows, not(test)))]
mod win;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    use std::time::Duration;

    use crate::test_support::work;

    #[test]
    fn the_tooltip_names_the_storage_and_fits_the_windows_limit() {
        assert_eq!(tooltip(Path::new(r"C:\Data\My Storage")), "FolderManager: watching My Storage");
        let long = tooltip(&Path::new(r"C:\").join("é".repeat(200)));
        assert_eq!(long.encode_utf16().count(), TIP_UNITS);
        assert!(long.starts_with("FolderManager: watching é") && long.ends_with("..."));
    }

    #[test]
    fn the_path_label_keeps_ampersands_visible() {
        assert_eq!(path_label(Path::new(r"C:\Tom & Ana\Store")), r"C:\Tom && Ana\Store");
        assert_eq!((OPEN_LABEL, STOP_LABEL), ("Open FolderManager", "Stop watching"));
    }

    #[test]
    fn a_tray_failure_is_a_status_and_the_watcher_keeps_running() {
        let dir = work("watch-tray");
        let root = dir.path().to_path_buf();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let handle = std::thread::spawn(move || super::super::watch(&root, &flag, &mut |_| {}));
        let failed = status(Err("no notification area".to_string()));
        std::thread::sleep(Duration::from_millis(500));
        assert_eq!(failed, TrayStatus::Unavailable("no notification area".to_string()));
        assert!(!stop.load(Ordering::Relaxed) && !handle.is_finished());
        stop.store(true, Ordering::Relaxed);
        handle.join().unwrap().unwrap();
    }
}
