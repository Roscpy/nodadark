use crate::state::AppState;
use anyhow::Result;
use nodadark_engine::{EngineEvent, InterceptedRequest};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

pub struct EngineClient {
    cmd_kind: CmdKind,
    event_rx: Option<mpsc::UnboundedReceiver<String>>,
}

enum CmdKind {
    Unix(tokio::net::unix::OwnedWriteHalf),
    Tcp(tokio::net::tcp::OwnedWriteHalf),
    Demo,
}

impl EngineClient {
    #[cfg(unix)]
    pub async fn connect_unix(path: &str) -> Result<Self> {
        let stream1 = tokio::net::UnixStream::connect(path).await?;
        let (r1, w1) = stream1.into_split();
        let mut lines1 = BufReader::new(r1).lines();
        let _ = lines1.next_line().await;

        let (tx, rx) = mpsc::unbounded_channel::<String>();
        let path_owned = path.to_string();
        tokio::spawn(async move {
            if let Ok(stream2) = tokio::net::UnixStream::connect(&path_owned).await {
                let (r2, mut w2) = stream2.into_split();
                let _ = w2.write_all(b"{\"command\":\"subscribe\"}\n").await;
                let mut lines2 = BufReader::new(r2).lines();
                while let Ok(Some(line)) = lines2.next_line().await {
                    if tx.send(line).is_err() { break; }
                }
            }
        });

        let (tx2, rx2) = mpsc::unbounded_channel::<String>();
        tokio::spawn(async move {
            while let Ok(Some(line)) = lines1.next_line().await {
                if tx2.send(line).is_err() { break; }
            }
        });

        let (tx_merged, rx_merged) = mpsc::unbounded_channel::<String>();
        let tx_m1 = tx_merged.clone();
        let tx_m2 = tx_merged.clone();
        tokio::spawn(async move {
            let mut rx = rx;
            while let Some(msg) = rx.recv().await {
                if tx_m1.send(msg).is_err() { break; }
            }
        });
        tokio::spawn(async move {
            let mut rx = rx2;
            while let Some(msg) = rx.recv().await {
                if tx_m2.send(msg).is_err() { break; }
            }
        });

        Ok(Self { cmd_kind: CmdKind::Unix(w1), event_rx: Some(rx_merged) })
    }

    pub async fn connect_tcp(addr: &str) -> Result<Self> {
        let stream1 = tokio::net::TcpStream::connect(addr).await?;
        let (r1, w1) = stream1.into_split();
        let mut lines1 = BufReader::new(r1).lines();
        let _ = lines1.next_line().await;

        let (tx, rx) = mpsc::unbounded_channel::<String>();
        let addr_owned = addr.to_string();
        tokio::spawn(async move {
            if let Ok(stream2) = tokio::net::TcpStream::connect(&addr_owned).await {
                let (r2, mut w2) = stream2.into_split();
                let _ = w2.write_all(b"{\"command\":\"subscribe\"}\n").await;
                let mut lines2 = BufReader::new(r2).lines();
                while let Ok(Some(line)) = lines2.next_line().await {
                    if tx.send(line).is_err() { break; }
                }
            }
        });

        let (tx2, rx2) = mpsc::unbounded_channel::<String>();
        tokio::spawn(async move {
            while let Ok(Some(line)) = lines1.next_line().await {
                if tx2.send(line).is_err() { break; }
            }
        });

        let (tx_merged, rx_merged) = mpsc::unbounded_channel::<String>();
        let tx_m1 = tx_merged.clone();
        let tx_m2 = tx_merged.clone();
        tokio::spawn(async move {
            let mut rx = rx;
            while let Some(msg) = rx.recv().await {
                if tx_m1.send(msg).is_err() { break; }
            }
        });
        tokio::spawn(async move {
            let mut rx = rx2;
            while let Some(msg) = rx.recv().await {
                if tx_m2.send(msg).is_err() { break; }
            }
        });

        Ok(Self { cmd_kind: CmdKind::Tcp(w1), event_rx: Some(rx_merged) })
    }

    pub fn demo_mode() -> Self {
        Self { cmd_kind: CmdKind::Demo, event_rx: None }
    }

    pub async fn send_command(&mut self, cmd: &serde_json::Value) {
        let json = serde_json::to_string(cmd).unwrap_or_default();
        let line = format!("{json}\n");
        match &mut self.cmd_kind {
            CmdKind::Unix(w) => { let _ = w.write_all(line.as_bytes()).await; }
            CmdKind::Tcp(w)  => { let _ = w.write_all(line.as_bytes()).await; }
            CmdKind::Demo    => {}
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

    pub async fn poll_messages(&mut self, app: &mut AppState) {
        let mut lines_to_process: Vec<String> = Vec::new();
        if let Some(rx) = &mut self.event_rx {
            for _ in 0..50 {
                match rx.try_recv() {
                    Ok(line) if !line.is_empty() => lines_to_process.push(line),
                    _ => break,
                }
            }
        }

        // Fix body vide — collecter les IDs des réponses complètes
        // pour auto-fetch le détail complet (headers + body)
        let mut ids_to_fetch: Vec<String> = Vec::new();
        for line in &lines_to_process {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                if v.get("type").and_then(|t| t.as_str()) == Some("response") {
                    if let Some(id) = v.get("id").and_then(|i| i.as_str()) {
                        ids_to_fetch.push(id.to_string());
                    }
                }
            }
        }

        // Traiter les messages
        for line in lines_to_process {
            self.process_line(&line, app);
        }

        // Auto-fetch le détail complet pour chaque requête terminée
        // → le body apparaît automatiquement sans appuyer sur i
        for id in ids_to_fetch {
            self.get_request(&id).await;
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
                    app.status_message = Some(
                        format!("Connecte -- Proxy :{}", app.proxy_port)
                    );
                }
                Some("ok") => {
                    app.status_message = resp["message"]
                        .as_str().map(|m| format!("OK: {m}"));
                }
                Some("error") => {
                    app.status_message = resp["message"]
                        .as_str().map(|m| format!("ERR: {m}"));
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
