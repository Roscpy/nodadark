// nodadark-desktop/src-tauri/src/main.rs
// Backend Tauri : pont entre le moteur Rust et l'interface Svelte

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod engine_bridge;

use engine_bridge::EngineState;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(EngineState::new())))
        .invoke_handler(tauri::generate_handler![
            commands::proxy::start_proxy,
            commands::proxy::stop_proxy,
            commands::proxy::toggle_pause,
            commands::proxy::get_status,
            commands::requests::list_requests,
            commands::requests::get_request,
            commands::requests::clear_requests,
            commands::replay::replay_request,
            commands::replay::drop_request,
        ])
        .setup(|app| {
            // Démarrer le proxy automatiquement sur le port par défaut
            let app_handle = app.handle();
            tokio::spawn(async move {
                // Démarrer en arrière-plan
                let _ = app_handle.emit_all("proxy-started", serde_json::json!({ "port": 8080 }));
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Erreur lors du lancement de Tauri");
}
