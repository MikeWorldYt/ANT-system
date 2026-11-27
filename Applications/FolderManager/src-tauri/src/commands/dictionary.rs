// The dictionary editor: its own window beside the main one, reading and autosaving a node's `.ant/dictionary.md`.
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use super::{log_failure, off_thread, run_locked, run_unlocked};
use crate::domain::error::{FmError, R};
use crate::services::automations::{dictionary, downloads};
use crate::services::automations::dictionary_lint::{lint, Issue};
use crate::services::dictionary_ai;

const LABEL: &str = "dictionary";
// WebView2 shares one environment per app, so a second window must ask for the same browser arguments as the main one or it fails to start.
pub(super) const BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --autoplay-policy=no-user-gesture-required --blink-settings=edgeWebRTCEnabled=false";

// One editor at a time: opening another folder's dictionary replaces it, so two windows never write the same file.
#[tauri::command]
pub async fn open_dictionary_window(app: AppHandle, folder: Vec<String>) -> R<()> {
    let fail = |e: tauri::Error| FmError::Internal { message: format!("Could not open the dictionary window: {e}") };
    if let Some(open) = app.get_webview_window(LABEL) {
        open.destroy().map_err(fail)?;
    }
    let target = serde_json::to_string(&folder).map_err(|e| FmError::Internal { message: e.to_string() })?;
    let title = if super::downloads::below_downloads(&folder).is_some() { "Dictionary - Downloads".to_string() } else { format!("Dictionary - /{}", folder.join("/")) };
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
    if super::downloads::below_downloads(&folder).is_some() {
        return downloads::read_text();
    }
    run_locked(app, move |root| dictionary::read_text(root, &folder)).await
}

#[tauri::command]
pub fn check_dictionary(text: String) -> Vec<Issue> {
    lint(&text)
}

// A file dropped on a group goes under that entry's `### files`, in the dictionary that holds the entry; no entry takes it out of every list.
#[tauri::command]
pub async fn pin_dictionary_file(app: AppHandle, folder: Vec<String>, entry: Option<String>, file: String) -> R<()> {
    if super::downloads::below_downloads(&folder).is_some() {
        crate::services::automations::pins::pin(&downloads::dictionary_path(), entry.as_deref(), &file)?;
        if let Err(e) = app.emit("fm://downloads-dictionary", ()) {
            eprintln!("Could not send fm://downloads-dictionary: {e}");
        }
        return Ok(());
    }
    run_locked(app, move |root| crate::services::automations::pins::pin_in_storage(root, &folder, entry.as_deref(), &file)).await
}

#[tauri::command]
pub async fn save_dictionary(app: AppHandle, folder: Vec<String>, text: String) -> R<Vec<Issue>> {
    if super::downloads::below_downloads(&folder).is_some() {
        downloads::write_text(&text)?;
        // The storage watcher never sees the temp folder, so the open batch is told here to read the dictionary again.
        if let Err(e) = app.emit("fm://downloads-dictionary", ()) {
            eprintln!("Could not send fm://downloads-dictionary: {e}");
        }
        return Ok(lint(&text));
    }
    run_locked(app, move |root| {
        dictionary::write_text(root, &folder, &text)?;
        Ok(lint(&text))
    })
    .await
}

// Claude Code may work for minutes, so no storage lock is held meanwhile; the editor saves the answer as it saves typing.
#[tauri::command]
pub async fn build_dictionary_ai(app: AppHandle, folder: Vec<String>, text: String) -> R<String> {
    let work = dictionary_ai::work_dir(&folder, super::downloads::below_downloads(&folder).is_some());
    let (dir, title) = match super::downloads::below_downloads(&folder) {
        Some(sub) => (crate::services::downloads::path_of(&super::downloads::base(&app)?, sub)?, "Downloads".to_string()),
        None => {
            let at = folder.clone();
            (run_unlocked(app, move |_, root| dictionary_ai::review_dir(root, &at)).await?, folder.last().cloned().unwrap_or_else(|| "the storage".to_string()))
        }
    };
    let result = off_thread(move || dictionary_ai::build(&dir, &work, &title, &text)).await?;
    log_failure(&result);
    result
}
