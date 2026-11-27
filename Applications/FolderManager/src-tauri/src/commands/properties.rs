// The Properties window: one native window per item, with the item's facts and a folder's totals computed off the UI thread.
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde_yaml::Mapping;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use super::{locked, off_thread};
use crate::app::storage_session::Storage;
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::resolve;
use crate::platform::volume::cluster_size;
use crate::services::downloads;
use crate::services::properties::{self, Facts, Totals};
use crate::services::tree::{review_links, tree_store};

const PREFIX: &str = "properties-";
const EVENT: &str = "fm://properties-totals";
const TICK: Duration = Duration::from_millis(250);

// The same item always maps to the same label, so a second request finds the window that is already open.
fn label_of(rel_parts: &[String]) -> String {
    let mut hasher = DefaultHasher::new();
    rel_parts.hash(&mut hasher);
    format!("{PREFIX}{:016x}", hasher.finish())
}

async fn target(app: &AppHandle, rel_parts: Vec<String>) -> R<PathBuf> {
    let app = app.clone();
    off_thread(move || {
        if let Some(path) = super::downloads::system_path(&app, &rel_parts) {
            return path;
        }
        locked(&app.state::<Storage>(), |root| {
            // Only a 998.Review link needs the tree to find its owner.
            let data = if review_links::is_link_path(&rel_parts) { tree_store::load(root)? } else { Mapping::new() };
            match review_links::link_folder(root, &data, &rel_parts)? {
                Some(dir) => Ok(dir),
                None => resolve(root, &rel_parts),
            }
        })
    })
    .await?
}

#[tauri::command]
pub async fn open_properties_window(app: AppHandle, rel_parts: Vec<String>, icon: String) -> R<()> {
    let fail = |e: tauri::Error| FmError::Internal { message: format!("Could not open the properties window: {e}") };
    let label = label_of(&rel_parts);
    if let Some(open) = app.get_webview_window(&label) {
        open.unminimize().map_err(fail)?;
        return open.set_focus().map_err(fail);
    }
    let name = rel_parts.last().map(|n| n.trim_start_matches(':')).unwrap_or_default();
    let title = format!("{name} Properties");
    let target = serde_json::to_string(&serde_json::json!({ "relParts": rel_parts, "icon": icon })).map_err(|e| FmError::Internal { message: e.to_string() })?;
    WebviewWindowBuilder::new(&app, label, WebviewUrl::App("index.html".into()))
        .title(title)
        .inner_size(320.0, 430.0)
        .resizable(false).maximizable(false)
        .theme(Some(tauri::Theme::Dark))
        .additional_browser_args(super::dictionary::BROWSER_ARGS)
        .initialization_script(format!("window.__PROPERTIES__ = {target};"))
        .build()
        .map(|_| ())
        .map_err(fail)
}

#[tauri::command]
pub async fn item_properties(app: AppHandle, rel_parts: Vec<String>) -> R<Facts> {
    let system_root = rel_parts.len() == 1 && downloads::split_root(&rel_parts).is_some();
    let path = target(&app, rel_parts).await?;
    off_thread(move || properties::facts(&path, system_root, cluster_size(&path))).await?
}

// Totals reach the window as events while the walk runs, so a big folder fills in instead of freezing; a closed window stops the walk.
#[tauri::command]
pub async fn item_totals(app: AppHandle, window: tauri::WebviewWindow, rel_parts: Vec<String>) -> R<Option<Totals>> {
    let path = target(&app, rel_parts).await?;
    let label = window.label().to_string();
    off_thread(move || {
        let mut last = Instant::now();
        properties::walk(&path, cluster_size(&path), &mut |partial| {
            if app.get_webview_window(&label).is_none() {
                return false;
            }
            if last.elapsed() >= TICK {
                last = Instant::now();
                if let Err(e) = app.emit_to(&label, EVENT, partial) {
                    eprintln!("Could not report the properties totals: {e}");
                }
            }
            true
        })
    })
    .await
}
