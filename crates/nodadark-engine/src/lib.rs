// nodadark-engine/src/lib.rs
// Public API du moteur NodaDark

pub mod api;
pub mod proxy;
pub mod rules;
pub mod storage;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;

// ────────────────────────────────────────────────────────────
//  Configuration
// ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Port d'écoute du proxy (défaut : 8080)
    pub port: u16,
    /// Adresse d'écoute (défaut : 127.0.0.1)
    pub bind: String,
    /// Port de l'API de contrôle (défaut : 9090)
    pub api_port: u16,
    /// Chemin du socket Unix (Linux/Android uniquement)
    pub socket_path: String,
    /// Répertoire des certificats CA
    pub cert_dir: String,
    /// Mode Fail-Open : laisser passer les certificats invalides
    pub fail_open: bool,
    /// Nombre max de requêtes en mémoire
    pub max_requests: usize,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        let cert_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("nodadark")
            .join("certs")
            .to_string_lossy()
            .to_string();

        Self {
            port: 8080,
            bind: "127.0.0.1".into(),
            api_port: 9090,
            socket_path: "/tmp/nodadark.sock".into(),
            cert_dir,
            fail_open: true,
            max_requests: 10_000,
        }
    }
}

// ────────────────────────────────────────────────────────────
//  Structures de données partagées
// ────────────────────────────────────────────────────────────

/// Une requête interceptée
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptedRequest {
    pub id: String,
    pub method: String,
    pub url: String,
    pub host: String,
    pub path: String,
    pub http_version: String,
    pub request_headers: Vec<(String, String)>,
    pub request_body: Option<Vec<u8>>,
    pub response_status: Option<u16>,
    pub response_headers: Vec<(String, String)>,
    pub response_body: Option<Vec<u8>>,
    pub duration_ms: Option<u64>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub state: RequestState,
    pub tls: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RequestState {
    Pending,   // Requête envoyée, réponse pas encore reçue
    Complete,  // Requête + réponse reçues
    Dropped,   // Requête droppée par une règle ou l'utilisateur
    Modified,  // Requête modifiée avant renvoi
    Error,     // Erreur réseau
}

// ────────────────────────────────────────────────────────────
//  Événements émis par le moteur (broadcast)
// ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EngineEvent {
    /// Nouvelle requête interceptée
    Request {
        id: String,
        method: String,
        url: String,
        host: String,
        timestamp: chrono::DateTime<chrono::Utc>,
        tls: bool,
    },
    /// Réponse reçue pour une requête
    Response {
        id: String,
        status: u16,
        duration_ms: u64,
        size: usize,
    },
    /// Requête droppée
    Dropped { id: String },
    /// Erreur sur une requête
    RequestError { id: String, error: String },
    /// Changement d'état du proxy
    ProxyState { paused: bool, port: u16 },
    /// Règle déclenchée
    RuleMatched { id: String, rule_name: String },
}

// ────────────────────────────────────────────────────────────
//  Moteur principal
// ────────────────────────────────────────────────────────────

pub struct ProxyEngine {
    config: Arc<ProxyConfig>,
    pub event_tx: broadcast::Sender<EngineEvent>,
    state: Arc<proxy::ProxyState>,
}

impl ProxyEngine {
    pub fn new(config: ProxyConfig) -> (Self, broadcast::Receiver<EngineEvent>) {
        let (tx, rx) = broadcast::channel(4096);
        let config = Arc::new(config);
        let state = Arc::new(proxy::ProxyState::new(
            config.max_requests,
            config.fail_open,
        ));

        (
            Self {
                config,
                event_tx: tx,
                state,
            },
            rx,
        )
    }

    /// Démarre le proxy et l'API de contrôle
    pub async fn start(self) -> Result<()> {
        let config = self.config.clone();
        let tx = self.event_tx.clone();
        let state = self.state.clone();

        // Générer / charger le certificat CA
        let ca = proxy::cert::CertificateAuthority::load_or_create(&config.cert_dir).await?;
        let ca = Arc::new(ca);

        tracing::info!(
            "🔒 NodaDark CA prêt : {}/nodadark-ca.crt",
            config.cert_dir
        );
        tracing::info!(
            "🚀 Proxy démarré sur {}:{}",
            config.bind,
            config.port
        );

        // Démarrer le serveur proxy HTTP
        let proxy_handle = {
            let config = config.clone();
            let tx = tx.clone();
            let state = state.clone();
            let ca = ca.clone();
            tokio::spawn(async move {
                proxy::server::run_proxy(config, state, ca, tx).await
            })
        };

        // Démarrer l'API de contrôle (socket Unix + TCP)
        let api_handle = {
            let config = config.clone();
            let tx = tx.clone();
            let state = state.clone();
            tokio::spawn(async move {
                api::server::run_api(config, state, tx).await
            })
        };

        tokio::select! {
            res = proxy_handle => {
                if let Ok(Err(e)) = res { tracing::error!("Proxy error: {e}"); }
            }
            res = api_handle => {
                if let Ok(Err(e)) = res { tracing::error!("API error: {e}"); }
            }
        }

        Ok(())
    }

    pub fn addr(&self) -> SocketAddr {
        format!("{}:{}", self.config.bind, self.config.port)
            .parse()
            .unwrap()
    }
}
