// nodadark-tui/src/network.rs
// Client de connexion au moteur NodaDark

use crate::state::AppState;
use anyhow::Result;
use nodadark_engine::{EngineEvent, InterceptedRequest};
use std::collections::VecDeque;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct EngineClient {
    kind: ClientKind,
}

enum ClientKind {
    Unix(UnixConn),
    Tcp(TcpConn),
    Demo,
}

struct UnixConn {
    writer: tokio::net::unix::OwnedWriteHalf,
    lines:  tokio::io::Lines<BufReader<tokio::net::unix::OwnedReadHalf>>,
}

struct TcpConn {
    writer: tokio::net::tcp::OwnedWriteHalf,
    lines:  tokio::io::Lines<BufReader<tokio::net::tcp::OwnedReadHalf>>,
}

impl EngineClient {
    #[cfg(unix)]
    pub async fn connect_unix(path: &str) -> Result<Self> {
        let stream = tokio::net::UnixStream::connect(path).await?;
        let (r, w) = stream.into_split();
        Ok(Self {
            kind: ClientKind::Unix(UnixConn {
                writer: w,
                lines:  BufReader::new(r).lines(),
            }),
        })
    }

    pub async fn connect_tcp(addr: &str) -> Result<Self> {
        let stream = tokio::net::TcpStream::connect(addr).await?;
        let (r, w) = stream.into_split();
        Ok(Self {
            kind: ClientKind::Tcp(TcpConn {
                writer: w,
                lines:  BufReader::new(r).lines(),
            }),
        })
    }

    pub fn demo_mode() -> Self {
        Self { kind: ClientKind::Demo }
    }

    pub async fn send_command(&mut self, cmd: &serde_json::Value) {
        let json = serde_json::to_string(cmd).unwrap_or_default();
        let line = format!("{json}\n");
        match &mut self.kind {
            ClientKind::Unix(c) => { let _ = c.writer.write_all(line.as_bytes()).await; }
            ClientKind::Tcp(c)  => { let _ = c.writer.write_all(line.as_bytes()).await; }
            ClientKind::Demo    => {}
        }
    }

    pub async fn pause(&mut self) {
        self.send_command(&serde_json::json!({"command":"pause"})).await;
    }
    pub async fn resume(&mut self) {
        self.send_command(&serde_json::json!({"command":"resume"})).await;
    }
    pub async fn drop_request(&mut self, id: &str) {
        self.send_command(&serde_json::json!({"command":"drop","id":id})).await;
    }
    pub async fn replay(&mut self, id: &str) {
        self.send_command(&serde_json::json!({"command":"replay","id":id})).await;
    }
    pub async fn request_list(&mut self, offset: usize, limit: usize) {
        self.send_command(&serde_json::json!({
            "command":"list_requests","offset":offset,"limit":limit
        })).await;
    }
    pub async fn get_request(&mut self, id: &str) {
        self.send_command(&serde_json::json!({"command":"get_request","id":id})).await;
    }
    pub async fn clear(&mut self) {
        self.send_command(&serde_json::json!({"command":"clear_requests"})).await;
    }

    /// Poll non-bloquant : lit les messages disponibles et met à jour l'état
    pub async fn poll_messages(&mut self, app: &mut AppState) {
        for _ in 0..20 {
            match self.try_read_line().await {
                Some(line) if !line.is_empty() => self.process_line(&line, app),
                _ => break,
            }
        }
    }

    async fn try_read_line(&mut self) -> Option<String> {
        use tokio::time::{timeout, Duration};
        match &mut self.kind {
            // Fix E0599 : .ok().and_then(|r| r.ok()).flatten()
            // pour dérouler Result<Result<Option<String>, io::Error>, Elapsed> -> Option<String>
            ClientKind::Unix(c) => {
                timeout(Duration::from_millis(1), c.lines.next_line())
                    .await
                    .ok()
                    .and_then(|res| res.ok())
                    .flatten()
            }
            ClientKind::Tcp(c) => {
                timeout(Duration::from_millis(1), c.lines.next_line())
                    .await
                    .ok()
                    .and_then(|res| res.ok())
                    .flatten()
            }
            ClientKind::Demo => None,
        }
    }

    fn process_line(&self, line: &str, app: &mut AppState) {
        if let Ok(resp) = serde_json::from_str::<serde_json::Value>(line) {
            match resp.get("type").and_then(|t| t.as_str()) {
                Some("requests") => {
                    if let Ok(items) = serde_json::from_value::<Vec<InterceptedRequest>>(
                        resp["items"].clone(),
                    ) {
                        for req in items { app.upsert_request(req); }
                    }
                }
                Some("request_detail") => {
                    if let Ok(req) = serde_json::from_value::<InterceptedRequest>(
                        resp["request"].clone(),
                    ) {
                        app.selected_request = Some(req.clone());
                        app.upsert_request(req);
                    }
                }
                Some("status") => {
                    app.proxy_paused     = resp["paused"].as_bool().unwrap_or(false);
                    app.proxy_port       = resp["port"].as_u64().unwrap_or(8080) as u16;
                    app.engine_connected = true;
                }
                Some("welcome") => {
                    app.engine_connected = true;
                    app.proxy_port = resp["proxy_port"].as_u64().unwrap_or(8080) as u16;
                    app.status_message = Some(format!("Connecte -- Proxy :{}", app.proxy_port));
                }
                Some("ok") => {
                    app.status_message = resp["message"].as_str().map(|m| format!("OK: {m}"));
                }
                Some("error") => {
                    app.status_message = resp["message"].as_str().map(|m| format!("ERR: {m}"));
                }
                _ => {
                    if let Ok(event) = serde_json::from_str::<EngineEvent>(line) {
                        app.handle_engine_event(event);
                    }
                }
            }
        }
    }
}
