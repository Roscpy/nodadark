// nodadark-engine/src/storage/mod.rs
// Sauvegarde et chargement de sessions sur disque

use crate::InterceptedRequest;
use anyhow::Result;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub struct SessionStorage {
    sessions_dir: PathBuf,
}

impl SessionStorage {
    pub fn new(sessions_dir: PathBuf) -> Self {
        Self { sessions_dir }
    }

    pub fn default_storage() -> Self {
        let dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nodadark")
            .join("sessions");
        Self::new(dir)
    }

    /// Sauvegarde une liste de requêtes dans un fichier .nds (NodaDark Session)
    pub async fn save_session(&self, name: &str, requests: &[InterceptedRequest]) -> Result<PathBuf> {
        fs::create_dir_all(&self.sessions_dir).await?;
        let filename = format!(
            "{}-{}.nds",
            name,
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        );
        let path = self.sessions_dir.join(&filename);

        let json = serde_json::to_string_pretty(requests)?;
        let mut file = fs::File::create(&path).await?;
        file.write_all(json.as_bytes()).await?;

        tracing::info!("💾 Session sauvegardée : {}", path.display());
        Ok(path)
    }

    /// Charge une session depuis un fichier .nds
    pub async fn load_session(&self, path: &PathBuf) -> Result<Vec<InterceptedRequest>> {
        let content = fs::read_to_string(path).await?;
        let requests: Vec<InterceptedRequest> = serde_json::from_str(&content)?;
        tracing::info!("📂 Session chargée : {} requêtes", requests.len());
        Ok(requests)
    }

    /// Liste les sessions disponibles
    pub async fn list_sessions(&self) -> Result<Vec<PathBuf>> {
        let mut sessions = vec![];
        if !self.sessions_dir.exists() {
            return Ok(sessions);
        }
        let mut entries = fs::read_dir(&self.sessions_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("nds") {
                sessions.push(path);
            }
        }
        sessions.sort();
        Ok(sessions)
    }

    /// Exporte les requêtes au format HAR (HTTP Archive)
    pub async fn export_har(&self, name: &str, requests: &[InterceptedRequest]) -> Result<PathBuf> {
        fs::create_dir_all(&self.sessions_dir).await?;
        let filename = format!(
            "{}-{}.har",
            name,
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        );
        let path = self.sessions_dir.join(&filename);

        let har = build_har(requests);
        let json = serde_json::to_string_pretty(&har)?;
        let mut file = fs::File::create(&path).await?;
        file.write_all(json.as_bytes()).await?;

        tracing::info!("📤 Export HAR : {}", path.display());
        Ok(path)
    }
}

// Génère un objet HAR minimaliste
fn build_har(requests: &[InterceptedRequest]) -> serde_json::Value {
    let entries: Vec<serde_json::Value> = requests
        .iter()
        .map(|r| {
            serde_json::json!({
                "startedDateTime": r.timestamp.to_rfc3339(),
                "time": r.duration_ms.unwrap_or(0),
                "request": {
                    "method": r.method,
                    "url": r.url,
                    "httpVersion": r.http_version,
                    "headers": r.request_headers.iter().map(|(k,v)| serde_json::json!({"name":k,"value":v})).collect::<Vec<_>>(),
                    "queryString": [],
                    "postData": r.request_body.as_ref().map(|b| serde_json::json!({"mimeType":"application/octet-stream","text": base64::encode(b)})),
                    "headersSize": -1,
                    "bodySize": r.request_body.as_ref().map(|b| b.len()).unwrap_or(0)
                },
                "response": {
                    "status": r.response_status.unwrap_or(0),
                    "statusText": "",
                    "httpVersion": r.http_version,
                    "headers": r.response_headers.iter().map(|(k,v)| serde_json::json!({"name":k,"value":v})).collect::<Vec<_>>(),
                    "content": {
                        "size": r.response_body.as_ref().map(|b| b.len()).unwrap_or(0),
                        "mimeType": "application/octet-stream",
                        "text": r.response_body.as_ref().map(|b| base64::encode(b)).unwrap_or_default()
                    },
                    "redirectURL": "",
                    "headersSize": -1,
                    "bodySize": r.response_body.as_ref().map(|b| b.len()).unwrap_or(0)
                },
                "cache": {},
                "timings": {"send": 0, "wait": r.duration_ms.unwrap_or(0), "receive": 0}
            })
        })
        .collect();

    serde_json::json!({
        "log": {
            "version": "1.2",
            "creator": {"name": "NodaDark", "version": "0.1.0"},
            "entries": entries
        }
    })
}
