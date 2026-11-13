// The dictionary editor: its own window beside the main one, reading and autosaving a node's `.ant/dictionary.md`.
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use super::run_locked;
use crate::domain::error::{FmError, R};
use crate::services::automations::dictionary;
use crate::services::automations::dictionary_lint::{lint, Issue};

const LABEL: &str = "dictionary";
// WebView2 shares one environment per app, so a second window must ask for the same browser arguments as the main one or it fails to start.
const BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --autoplay-policy=no-user-gesture-required --blink-settings=edgeWebRTCEnabled=false";

// One editor at a time: opening another folder's dictionary replaces it, so two windows never write the same file.
#[tauri::command]
pub async fn open_dictionary_window(app: AppHandle, folder: Vec<String>) -> R<()> {
    let fail = |e: tauri::Error| FmError::Internal { message: format!("Could not open the dictionary window: {e}") };
    if let Some(open) = app.get_webview_window(LABEL) {
        open.destroy().map_err(fail)?;
    }
    let target = serde_json::to_string(&folder).map_err(|e| FmError::Internal { message: e.to_string() })?;
    let title = format!("Dictionary - /{}", folder.join("/"));
    WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::App("index.html".into()))
        .title(title)
        .inner_size(560.0, 640.0)
        .min_inner_size(360.0, 240.0)
        .theme(Some(tauri::Theme::Dark))
        .additional_browser_args(BROWSER_ARGS)
        .initialization_script(format!("window.__DICTIONARY__ = {target};"))
        .build()
        .map(|_| ())
        .map_err(fail)
}

#[tauri::command]
pub async fn read_dictionary(app: AppHandle, folder: Vec<String>) -> R<String> {
    run_locked(app, move |root| dictionary::read_text(root, &folder)).await
}

#[tauri::command]
pub fn check_dictionary(text: String) -> Vec<Issue> {
    lint(&text)
}

#[tauri::command]
pub async fn save_dictionary(app: AppHandle, folder: Vec<String>, text: String) -> R<Vec<Issue>> {
    run_locked(app, move |root| {
        dictionary::write_text(root, &folder, &text)?;
        Ok(lint(&text))
    })
    .await
}
