// nodadark-desktop/src-tauri/src/commands/proxy.rs

use crate::engine_bridge::EngineState;
use std::sync::Arc;
use tauri::{State, Window};
use tokio::sync::Mutex;

type EngineStateHandle = Arc<Mutex<EngineState>>;

#[tauri::command]
pub async fn start_proxy(
    port: u16,
    window: Window,
    state: State<'_, EngineStateHandle>,
) -> Result<String, String> {
    let mut s = state.lock().await;
    s.start(port, window).await?;
    Ok(format!("Proxy démarré sur le port {port}"))
}

#[tauri::command]
pub async fn stop_proxy(state: State<'_, EngineStateHandle>) -> Result<String, String> {
    let mut s = state.lock().await;
    s.running = false;
    s.engine_tx = None;
    Ok("Proxy arrêté".into())
}

#[tauri::command]
pub async fn toggle_pause(state: State<'_, EngineStateHandle>) -> Result<bool, String> {
    // TODO: envoyer la commande pause/resume via l'API socket
    Ok(false)
}

#[tauri::command]
pub async fn get_status(
    state: State<'_, EngineStateHandle>,
) -> Result<serde_json::Value, String> {
    let s = state.lock().await;
    Ok(serde_json::json!({
        "running": s.running,
        "port": s.config.port,
        "paused": false,
        "ca_path": format!("{}/nodadark-ca.crt", s.config.cert_dir),
    }))
}
