// The webview's answer to a request an agent sent through the UI bridge.
use serde_json::Value;
use tauri::{AppHandle, Manager};

use crate::app::ui_bridge::UiBridge;
use crate::domain::error::{FmError, R};

#[tauri::command]
pub fn answer_ui_request(app: AppHandle, id: u64, value: Option<Value>, error: Option<String>) -> R<()> {
    let bridge = app.try_state::<UiBridge>().ok_or_else(|| FmError::Internal { message: "The UI bridge is not running".to_string() })?;
    bridge.answer(id, error.map_or_else(|| Ok(value.unwrap_or(Value::Null)), Err));
    Ok(())
}
