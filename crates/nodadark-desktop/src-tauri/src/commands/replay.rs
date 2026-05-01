// nodadark-desktop/src-tauri/src/commands/replay.rs

use crate::engine_bridge::EngineState;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

type EngineStateHandle = Arc<Mutex<EngineState>>;

#[tauri::command]
pub async fn replay_request(
    id: String,
    modified_headers: Option<HashMap<String, String>>,
    modified_body: Option<String>,
    state: State<'_, EngineStateHandle>,
) -> Result<String, String> {
    // TODO: envoyer la commande replay au moteur via socket Unix / TCP
    tracing::info!("Replay demandé pour {id}");
    Ok(format!("Replay planifié pour {id}"))
}

#[tauri::command]
pub async fn drop_request(
    id: String,
    state: State<'_, EngineStateHandle>,
) -> Result<String, String> {
    tracing::info!("Drop demandé pour {id}");
    Ok(format!("Requête {id} droppée"))
}
