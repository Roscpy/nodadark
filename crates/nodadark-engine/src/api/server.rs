// nodadark-engine/src/api/server.rs
// NodaDark v0.1.5 — Replay fonctionnel

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

async fn run_unix_api(
    config: Arc<ProxyConfig>,
    state: Arc<ProxyState>,
    tx: broadcast::Sender<EngineEvent>,
) -> Result<()> {
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
            let welcome = ApiResponse::Welcome {
                version: "0.1.5".into(),
                proxy_port: config.port,
                api_port: config.api_port,
            };
            let _ = send_response(&mut writer, &welcome).await;

            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() { continue; }
                match serde_json::from_str::<ApiCommand>(&line) {
                    Ok(cmd) => {
                        let responses = handle_command(cmd, &state, &tx, &config).await;
                        for resp in responses {
                            if send_response(&mut writer, &resp).await.is_err() { break; }
                        }
                    }
                    Err(e) => {
                        let _ = send_response(&mut writer,
                            &ApiResponse::Error { message: format!("JSON invalide: {e}") }).await;
                    }
                }
            }
        });
    }
}

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
                version: "0.1.5".into(),
                proxy_port: config.port,
                api_port: config.api_port,
            };
            let _ = send_response(&mut writer, &welcome).await;

            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() { continue; }
                match serde_json::from_str::<ApiCommand>(&line) {
                    Ok(ApiCommand::Subscribe) => {
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
                            if send_response(&mut writer, &resp).await.is_err() { break; }
                        }
                    }
                    Err(e) => {
                        let _ = send_response(&mut writer,
                            &ApiResponse::Error { message: format!("JSON invalide: {e}") }).await;
                    }
                }
            }
        });
    }
}

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
                    .filter(|r| r.url.to_lowercase().contains(&f)
                        || r.host.to_lowercase().contains(&f))
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
                None => vec![ApiResponse::Error {
                    message: format!("Requête {id} introuvable") }],
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
                Ok(path) => vec![ApiResponse::Saved {
                    path: path.to_string_lossy().into() }],
                Err(e) => vec![ApiResponse::Error { message: e.to_string() }],
            }
        }

        ApiCommand::ExportHar { name } => {
            let name = name.unwrap_or_else(|| "export".into());
            let requests = state.list(0, usize::MAX);
            let storage = crate::storage::SessionStorage::default_storage();
            match storage.export_har(&name, &requests).await {
                Ok(path) => vec![ApiResponse::Saved {
                    path: path.to_string_lossy().into() }],
                Err(e) => vec![ApiResponse::Error { message: e.to_string() }],
            }
        }

        // ─── REPLAY FONCTIONNEL v0.1.5 ──────────────────────────
        ApiCommand::Replay { id, modified_headers, modified_body } => {
            match state.get(&id) {
                None => vec![ApiResponse::Error {
                    message: format!("Requête {id} introuvable") }],

                Some(original) => {
                    let state2   = state.clone();
                    let tx2      = tx.clone();
                    let new_id   = uuid::Uuid::new_v4().to_string();
                    let method   = original.method.clone();
                    let url      = original.url.clone();
                    let host     = original.host.clone();
                    let tls      = original.tls;

                    // Construire les headers — appliquer les modifications
                    let mut headers = original.request_headers.clone();
                    for (k, v) in modified_headers {
                        if let Some(pos) = headers.iter().position(|(hk, _)|
                            hk.to_lowercase() == k.to_lowercase()) {
                            headers[pos].1 = v;
                        } else {
                            headers.push((k, v));
                        }
                    }

                    // Body : modifié ou original
                    let body = modified_body
                        .map(|b| b.into_bytes())
                        .or_else(|| original.request_body.clone());

                    tokio::spawn(async move {
                        // Client HTTP qui ignore les erreurs SSL
                        // (on passe par le vrai réseau, pas par notre proxy)
                        let client = match reqwest::Client::builder()
                            .danger_accept_invalid_certs(true)
                            .timeout(std::time::Duration::from_secs(30))
                            .build() {
                            Ok(c) => c,
                            Err(e) => {
                                tracing::error!("Replay — client HTTP: {e}");
                                return;
                            }
                        };

                        // Méthode HTTP
                        let http_method = match method.as_str() {
                            "POST"    => reqwest::Method::POST,
                            "PUT"     => reqwest::Method::PUT,
                            "DELETE"  => reqwest::Method::DELETE,
                            "PATCH"   => reqwest::Method::PATCH,
                            "HEAD"    => reqwest::Method::HEAD,
                            "OPTIONS" => reqwest::Method::OPTIONS,
                            _         => reqwest::Method::GET,
                        };

                        let mut rb = client.request(http_method, &url);

                        // Ajouter les headers (sauf host — géré par reqwest)
                        for (k, v) in &headers {
                            let kl = k.to_lowercase();
                            if kl != "host" && kl != "content-length" {
                                rb = rb.header(k.as_str(), v.as_str());
                            }
                        }

                        // Ajouter le body si présent
                        if let Some(b) = body {
                            rb = rb.body(b);
                        }

                        let timestamp = chrono::Utc::now();
                        let start     = std::time::Instant::now();

                        // Émettre l'événement "nouvelle requête"
                        let _ = tx2.send(crate::EngineEvent::Request {
                            id: new_id.clone(),
                            method: method.clone(),
                            url: url.clone(),
                            host: host.clone(),
                            timestamp,
                            tls,
                        });

                        match rb.send().await {
                            Ok(resp) => {
                                let status      = resp.status().as_u16();
                                let duration_ms = start.elapsed().as_millis() as u64;
                                let resp_headers: Vec<(String, String)> = resp
                                    .headers()
                                    .iter()
                                    .map(|(k, v)| (
                                        k.to_string(),
                                        v.to_str().unwrap_or("").to_string(),
                                    ))
                                    .collect();

                                let body_bytes = resp.bytes().await
                                    .unwrap_or_default().to_vec();
                                let size = body_bytes.len();

                                // Sauvegarder le résultat du replay
                                let replayed = crate::InterceptedRequest {
                                    id: new_id.clone(),
                                    method,
                                    url,
                                    host,
                                    path: String::new(),
                                    http_version: "HTTP/1.1".into(),
                                    request_headers: headers,
                                    request_body: None,
                                    response_status: Some(status),
                                    response_headers: resp_headers,
                                    response_body: Some(body_bytes),
                                    duration_ms: Some(duration_ms),
                                    timestamp,
                                    state: crate::RequestState::Modified,
                                    tls,
                                    error: None,
                                };
                                state2.upsert(replayed);

                                let _ = tx2.send(crate::EngineEvent::Response {
                                    id: new_id,
                                    status,
                                    duration_ms,
                                    size,
                                });
                            }
                            Err(e) => {
                                tracing::error!("Replay échoué: {e}");
                                let _ = tx2.send(crate::EngineEvent::RequestError {
                                    id: new_id,
                                    error: e.to_string(),
                                });
                            }
                        }
                    });

                    vec![ApiResponse::Ok {
                        message: "↪ Replay lancé — nouvelle requête en cours".into() }]
                }
            }
        }

        ApiCommand::Subscribe => vec![],
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
