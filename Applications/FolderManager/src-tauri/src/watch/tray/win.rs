// Windows tray: the icon, its menu and the message loop that the icon's hidden window needs.
use std::ffi::c_void;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIconBuilder, TrayIconEvent};

use super::{log, path_label, tooltip, OPEN_LABEL, STOP_LABEL};

// tauri-build embeds the app icon under this id, so the exe needs no icon file beside it.
const APP_ICON: u16 = 32512;
const DETACHED_PROCESS: u32 = 0x0000_0008;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
const PM_REMOVE: u32 = 0x0001;
const QS_ALLINPUT: u32 = 0x04FF;
const WM_QUIT: u32 = 0x0012;
const WAIT_FAILED: u32 = 0xFFFF_FFFF;
const WAIT_MS: u32 = 200;

#[repr(C)]
struct Msg {
    hwnd: *mut c_void,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    x: i32,
    y: i32,
}

#[link(name = "user32")]
extern "system" {
    fn PeekMessageW(msg: *mut Msg, hwnd: *mut c_void, min: u32, max: u32, remove: u32) -> i32;
    fn TranslateMessage(msg: *const Msg) -> i32;
    fn DispatchMessageW(msg: *const Msg) -> isize;
    fn MsgWaitForMultipleObjects(count: u32, handles: *const *mut c_void, all: i32, ms: u32, wake: u32) -> u32;
}

pub fn run<T>(root: &Path, stop: &AtomicBool, watcher: &JoinHandle<T>) -> Result<(), String> {
    let open = MenuItem::new(OPEN_LABEL, true, None);
    let quit = MenuItem::new(STOP_LABEL, true, None);
    let menu = Menu::new();
    let path = MenuItem::new(path_label(root), false, None);
    menu.append_items(&[&path, &open, &quit]).map_err(|e| e.to_string())?;
    let icon = Icon::from_resource(APP_ICON, None).map_err(|e| e.to_string())?;
    // Hover and click events are unused; a no-op handler keeps them from queueing all session long.
    TrayIconEvent::set_event_handler(Some(|_: TrayIconEvent| {}));
    let _tray = TrayIconBuilder::new()
        .with_icon(icon)
        .with_tooltip(tooltip(root))
        .with_menu(Box::new(menu))
        .build()
        .map_err(|e| e.to_string())?;
    while !stop.load(Ordering::Relaxed) && !watcher.is_finished() {
        if !pump() {
            stop.store(true, Ordering::Relaxed);
        }
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == *open.id() {
                open_gui();
            } else if event.id == *quit.id() {
                log("info", "Watch: stopped from the tray");
                stop.store(true, Ordering::Relaxed);
            }
        }
    }
    Ok(())
}

// Returns false once the thread received WM_QUIT.
fn pump() -> bool {
    let mut msg = Msg { hwnd: null_mut(), message: 0, wparam: 0, lparam: 0, time: 0, x: 0, y: 0 };
    // SAFETY: plain user32 calls on this thread's own queue with a valid, owned MSG buffer.
    unsafe {
        if MsgWaitForMultipleObjects(0, null(), 0, WAIT_MS, QS_ALLINPUT) == WAIT_FAILED {
            std::thread::sleep(Duration::from_millis(WAIT_MS.into()));
        }
        while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
            if msg.message == WM_QUIT {
                return false;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    true
}

// The GUI is the same exe started without `--watch`; detached so closing it never touches this watcher.
fn open_gui() {
    let started = std::env::current_exe().and_then(|exe| {
        Command::new(exe)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
            .spawn()
    });
    match started {
        Ok(_) => log("info", "Watch: opened FolderManager from the tray"),
        Err(e) => log("error", &format!("Watch: could not open FolderManager: {e}")),
    }
}
