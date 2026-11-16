// The window's end of the UI bridge: hands each authenticated request to the webview and waits for its answer.
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, UserAttentionType};

use crate::app::storage_session::Storage;
use crate::services::bridge::endpoint::{self, Endpoint};
use crate::services::bridge::{server, Reply, UiCall};

const UI_REQUEST: &str = "fm://ui-request";
const WINDOW: &str = "main";

type Pending = Arc<Mutex<HashMap<u64, Sender<Reply>>>>;

pub struct UiBridge {
    file: PathBuf,
    token: String,
    pending: Pending,
}

impl UiBridge {
    pub fn start(app: &AppHandle) -> Result<Self, String> {
        let data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        let file = endpoint::file(&data_dir, &std::env::current_exe().map_err(|e| e.to_string())?);
        let token = endpoint::new_token().map_err(|e| format!("no session token: {e}"))?;
        let (listener, port) = server::bind().map_err(|e| format!("no local port: {e}"))?;
        endpoint::write(&file, &Endpoint { port, token: token.clone() }).map_err(|e| format!("{}: {e}", file.display()))?;
        let pending = Pending::default();
        let (handle, waiting, secret) = (app.clone(), pending.clone(), token.clone());
        std::thread::spawn(move || {
            let open = || handle.state::<Storage>().root().ok();
            server::serve(listener, secret, open, |call| ask(&handle, &waiting, call));
        });
        Ok(Self { file, token, pending })
    }

    pub fn answer(&self, id: u64, reply: Reply) {
        let Some(waiter) = self.pending.lock().unwrap_or_else(PoisonError::into_inner).remove(&id) else { return };
        if waiter.send(reply).is_err() {
            eprintln!("UI bridge: answer {id} came after its request timed out");
        }
    }

    pub fn stop(&self) {
        endpoint::remove(&self.file, &self.token);
    }
}

fn ask(app: &AppHandle, pending: &Pending, call: UiCall) -> Reply {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let (tx, rx) = channel();
    let lock = || pending.lock().unwrap_or_else(PoisonError::into_inner);
    lock().insert(id, tx);
    // Sent to the main window only, not to every webview the app may open.
    let wait = call.wait();
    if matches!(call, UiCall::Confirm { .. }) {
        draw_attention(app);
    }
    let sent = app.emit_to(WINDOW, UI_REQUEST, json!({ "id": id, "call": call }));
    let reply = match sent {
        Ok(()) => rx.recv_timeout(wait).unwrap_or_else(|_| Err(format!("Folder Manager did not answer within {} s; its window may still be loading or no one answered", wait.as_secs()))),
        Err(e) => Err(format!("Could not reach the Folder Manager window: {e}")),
    };
    lock().remove(&id);
    reply
}

#[derive(Debug, PartialEq)]
struct Attention {
    show: bool,
    unminimize: bool,
    flash: bool,
}

fn attention(visible: bool, minimized: bool, focused: bool) -> Attention {
    Attention { show: !visible, unminimize: minimized, flash: !visible || minimized || !focused }
}

// A confirmation nobody sees expires as a no, so a hidden, minimized or unfocused window first asks to be looked at.
fn draw_attention(app: &AppHandle) {
    let Some(window) = app.get_webview_window(WINDOW) else { return };
    let steps = attention(window.is_visible().unwrap_or(true), window.is_minimized().unwrap_or(false), window.is_focused().unwrap_or(false));
    let result = (|| -> tauri::Result<()> {
        if steps.show {
            window.show()?;
        }
        if steps.unminimize {
            window.unminimize()?;
        }
        if steps.flash {
            window.request_user_attention(Some(UserAttentionType::Critical))?;
        }
        Ok(())
    })();
    if let Err(e) = result {
        eprintln!("Could not draw attention to the agent's confirmation: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unseen_window_is_shown_and_flashed_and_a_focused_one_is_left_alone() {
        assert_eq!(attention(true, false, true), Attention { show: false, unminimize: false, flash: false });
        assert_eq!(attention(true, false, false), Attention { show: false, unminimize: false, flash: true });
        assert_eq!(attention(true, true, false), Attention { show: false, unminimize: true, flash: true });
        assert_eq!(attention(false, false, false), Attention { show: true, unminimize: false, flash: true });
    }
}
