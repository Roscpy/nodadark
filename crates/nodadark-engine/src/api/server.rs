// nodadark-engine/src/api/server.rs
// Serveur de l'API de contrôle (Unix socket + TCP)

use super::protocol::{ApiCommand, ApiResponse};
use crate::{proxy::ProxyState, EngineEvent, ProxyConfig};
use anyhow::Result;
use std::sync::Arc;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, UnixListener},
    sync::broadcast,
};

pub async fn run_api(
    config: Arc<ProxyConfig>,
    state: Arc<ProxyState>,
    tx: broadcast::Sender<EngineEvent>,
) -> Result<()> {
    // Démarrer les deux interfaces en parallèle
    let unix_handle = {
        let config = config.clone();
        let state  = state.clone();
        let tx     = tx.clone();
        tokio::spawn(run_unix_api(config, state, tx))
    };

    let tcp_handle = {
        let config = config.clone();
        let state  = state.clone();
        let tx     = tx.clone();
        tokio::spawn(run_tcp_api(config, state, tx))
    };

    tokio::select! {
        _ = unix_handle => {}
        _ = tcp_handle  => {}
    }

    Ok(())
}

// ────────────────────────────────────────────────────────────
//  Socket Unix (Linux / Android / macOS)
// ────────────────────────────────────────────────────────────

async fn run_unix_api(
    config: Arc<ProxyConfig>,
    state: Arc<ProxyState>,
    tx: broadcast::Sender<EngineEvent>,
) -> Result<()> {
    // Supprimer le socket s'il existe déjà
    let _ = tokio::fs::remove_file(&config.socket_path).await;

    let listener = UnixListener::bind(&config.socket_path)?;
    tracing::info!("🔌 API Unix socket : {}", config.socket_path);

    loop {
        let (stream, _) = listener.accept().await?;
        let state  = state.clone();
        let tx     = tx.clone();
        let config = config.clone();

        tokio::spawn(async move {
            let (reader, mut writer) = tokio::io::split(stream);
            let mut lines = BufReader::new(reader).lines();

            // Message de bienvenue
            let welcome = ApiResponse::Welcome {
                version: "0.1.0".into(),
                proxy_port: config.port,
                api_port: config.api_port,
            };
            let _ = send_response(&mut writer, &welcome).await;

            // Recevoir et traiter les commandes
            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<ApiCommand>(&line) {
                    Ok(cmd) => {
                        let responses = handle_command(cmd, &state, &tx, &config).await;
                        for resp in responses {
                            if send_response(&mut writer, &resp).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = send_response(
                            &mut writer,
                            &ApiResponse::Error { message: format!("JSON invalide: {e}") },
                        ).await;
                    }
                }
            }
        });
    }
}

// ────────────────────────────────────────────────────────────
//  TCP (port 9090 — pour Windows et connexions distantes)
// ────────────────────────────────────────────────────────────

async fn run_tcp_api(
    config: Arc<ProxyConfig>,
    state: Arc<ProxyState>,
    tx: broadcast::Sender<EngineEvent>,
) -> Result<()> {
    let addr = format!("127.0.0.1:{}", config.api_port);
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("🔌 API TCP : {addr}");

    loop {
        let (stream, peer) = listener.accept().await?;
        tracing::debug!("API client connecté depuis {peer}");
        let state  = state.clone();
        let tx     = tx.clone();
        let config = config.clone();

        tokio::spawn(async move {
            let (reader, mut writer) = stream.into_split();
            let mut lines = BufReader::new(reader).lines();

            let welcome = ApiResponse::Welcome {
                version: "0.1.0".into(),
                proxy_port: config.port,
                api_port: config.api_port,
            };
            let _ = send_response(&mut writer, &welcome).await;

            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<ApiCommand>(&line) {
                    Ok(ApiCommand::Subscribe) => {
                        // Diffuser les événements en temps réel
                        let mut rx = tx.subscribe();
                        loop {
                            match rx.recv().await {
                                Ok(event) => {
                                    let json = serde_json::to_string(&event).unwrap_or_default();
                                    if writer.write_all(format!("{json}\n").as_bytes()).await.is_err() {
                                        break;
                                    }
                                }
                                Err(_) => break,
                            }
                        }
                        return;
                    }
                    Ok(cmd) => {
                        let responses = handle_command(cmd, &state, &tx, &config).await;
                        for resp in responses {
                            if send_response(&mut writer, &resp).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = send_response(
                            &mut writer,
                            &ApiResponse::Error { message: format!("JSON invalide: {e}") },
                        ).await;
                    }
                }
            }
        });
    }
}

// ────────────────────────────────────────────────────────────
//  Traitement des commandes
// ────────────────────────────────────────────────────────────

async fn handle_command(
    cmd: ApiCommand,
    state: &Arc<ProxyState>,
    tx: &broadcast::Sender<EngineEvent>,
    config: &Arc<ProxyConfig>,
) -> Vec<ApiResponse> {
    match cmd {
        ApiCommand::Pause => {
            state.set_paused(true);
            let _ = tx.send(EngineEvent::ProxyState { paused: true, port: config.port });
            vec![ApiResponse::Ok { message: "Proxy mis en pause".into() }]
        }

        ApiCommand::Resume => {
            state.set_paused(false);
            let _ = tx.send(EngineEvent::ProxyState { paused: false, port: config.port });
            vec![ApiResponse::Ok { message: "Proxy repris".into() }]
        }

        ApiCommand::Drop { id } => {
            if state.drop_request(&id) {
                let _ = tx.send(EngineEvent::Dropped { id: id.clone() });
                vec![ApiResponse::Ok { message: format!("Requête {id} droppée") }]
            } else {
                vec![ApiResponse::Error { message: format!("Requête {id} introuvable") }]
            }
        }

        ApiCommand::ListRequests { offset, limit, filter } => {
            let all = state.list(offset, limit);
            let filtered: Vec<_> = if let Some(f) = &filter {
                let f = f.to_lowercase();
                all.into_iter()
                    .filter(|r| r.url.to_lowercase().contains(&f) || r.host.to_lowercase().contains(&f))
                    .collect()
            } else {
                all
            };
            let total = state.count();
            vec![ApiResponse::Requests { items: filtered, total }]
        }

        ApiCommand::GetRequest { id } => {
            match state.get(&id) {
                Some(req) => vec![ApiResponse::RequestDetail { request: req }],
                None => vec![ApiResponse::Error { message: format!("Requête {id} introuvable") }],
            }
        }

        ApiCommand::ClearRequests => {
            state.clear();
            vec![ApiResponse::Ok { message: "Toutes les requêtes effacées".into() }]
        }

        ApiCommand::Status => {
            vec![ApiResponse::Status {
                paused: state.is_paused(),
                port: config.port,
                request_count: state.count(),
                ca_path: format!("{}/nodadark-ca.crt", config.cert_dir),
            }]
        }

        ApiCommand::SaveSession { name } => {
            let name = name.unwrap_or_else(|| "session".into());
            let requests = state.list(0, usize::MAX);
            let storage = crate::storage::SessionStorage::default_storage();
            match storage.save_session(&name, &requests).await {
                Ok(path) => vec![ApiResponse::Saved { path: path.to_string_lossy().into() }],
                Err(e) => vec![ApiResponse::Error { message: e.to_string() }],
            }
        }

        ApiCommand::ExportHar { name } => {
            let name = name.unwrap_or_else(|| "export".into());
            let requests = state.list(0, usize::MAX);
            let storage = crate::storage::SessionStorage::default_storage();
            match storage.export_har(&name, &requests).await {
                Ok(path) => vec![ApiResponse::Saved { path: path.to_string_lossy().into() }],
                Err(e) => vec![ApiResponse::Error { message: e.to_string() }],
            }
        }

        ApiCommand::Replay { id, modified_headers, modified_body } => {
            // TODO: implémenter le replay dans Phase 2+
            vec![ApiResponse::Ok { message: format!("Replay de {id} planifié (à implémenter)") }]
        }

        ApiCommand::Subscribe => {
            // Géré directement dans le handler de connexion
            vec![]
        }
    }
}

async fn send_response<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    response: &ApiResponse,
) -> Result<()> {
    let json = serde_json::to_string(response)?;
    writer.write_all(format!("{json}\n").as_bytes()).await?;
    Ok(())
}
