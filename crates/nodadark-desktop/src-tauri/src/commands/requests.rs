// nodadark-desktop/src-tauri/src/commands/requests.rs

use crate::engine_bridge::EngineState;
use nodadark_engine::InterceptedRequest;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

type EngineStateHandle = Arc<Mutex<EngineState>>;

#[tauri::command]
pub async fn list_requests(
    offset: Option<usize>,
    limit: Option<usize>,
    filter: Option<String>,
    state: State<'_, EngineStateHandle>,
) -> Result<serde_json::Value, String> {
    // Dans l'implémentation finale, on interroge le moteur via socket
    // Pour l'instant on retourne un exemple de structure
    Ok(serde_json::json!({
        "items": [],
        "total": 0,
        "offset": offset.unwrap_or(0),
        "limit": limit.unwrap_or(100),
    }))
}

#[tauri::command]
pub async fn get_request(
    id: String,
    state: State<'_, EngineStateHandle>,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({ "id": id, "found": false }))
}

#[tauri::command]
pub async fn clear_requests(
    state: State<'_, EngineStateHandle>,
) -> Result<String, String> {
    Ok("Requêtes effacées".into())
}
