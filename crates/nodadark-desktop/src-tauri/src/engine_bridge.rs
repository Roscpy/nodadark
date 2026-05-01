// nodadark-desktop/src-tauri/src/engine_bridge.rs
// Pont entre le backend Tauri et nodadark-engine

use nodadark_engine::{EngineEvent, ProxyConfig, ProxyEngine};
use std::sync::Arc;
use tauri::Window;
use tokio::sync::{broadcast, Mutex};

pub struct EngineState {
    pub engine_tx: Option<broadcast::Sender<EngineEvent>>,
    pub config: ProxyConfig,
    pub running: bool,
}

impl EngineState {
    pub fn new() -> Self {
        Self {
            engine_tx: None,
            config: ProxyConfig::default(),
            running: false,
        }
    }

    pub async fn start(&mut self, port: u16, window: Window) -> Result<(), String> {
        if self.running {
            return Err("Proxy déjà démarré".into());
        }

        let config = ProxyConfig {
            port,
            ..Default::default()
        };

        let (engine, mut rx) = ProxyEngine::new(config.clone());
        let tx = engine.event_tx.clone();

        // Diffuser les événements vers le frontend Svelte
        let win = window.clone();
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        let _ = win.emit("engine-event", &event);
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        eprintln!("⚠ {n} événements perdus");
                    }
                    Err(_) => break,
                }
            }
        });

        self.engine_tx = Some(tx);
        self.config = config;
        self.running = true;

        // Démarrer le moteur en arrière-plan
        tokio::spawn(async move {
            if let Err(e) = engine.start().await {
                eprintln!("Erreur moteur : {e}");
            }
        });

        Ok(())
    }
}
