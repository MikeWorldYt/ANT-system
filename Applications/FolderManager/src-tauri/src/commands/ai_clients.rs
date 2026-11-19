// Commands that connect the MCP server to the user's AI clients.
use super::off_thread;
use crate::domain::error::R;
use crate::services::ai_clients::{self, AiClient};

#[tauri::command]
pub async fn ai_clients() -> R<Vec<AiClient>> {
    off_thread(ai_clients::list).await?
}

#[tauri::command]
pub async fn ai_connect(id: String) -> R<AiClient> {
    off_thread(move || ai_clients::connect(&id)).await?
}

#[tauri::command]
pub async fn ai_disconnect(id: String) -> R<AiClient> {
    off_thread(move || ai_clients::disconnect(&id)).await?
}

#[tauri::command]
pub async fn ai_restart_client(id: String) -> R<AiClient> {
    off_thread(move || ai_clients::restart(&id)).await?
}

#[tauri::command]
pub async fn ai_config_snippet() -> R<String> {
    off_thread(ai_clients::config_snippet).await?
}
