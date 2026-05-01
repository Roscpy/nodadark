// nodadark-engine/src/api/protocol.rs
// Protocole de communication entre le moteur et les UIs (JSON-lines)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Commandes envoyées par une UI vers le moteur
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum ApiCommand {
    /// Mettre en pause le proxy
    Pause,
    /// Reprendre le proxy
    Resume,
    /// Dropper une requête en attente
    Drop { id: String },
    /// Rejouer une requête (optionnellement modifiée)
    Replay {
        id: String,
        #[serde(default)]
        modified_headers: HashMap<String, String>,
        modified_body: Option<String>,
    },
    /// Récupérer la liste des requêtes paginée
    ListRequests {
        #[serde(default)]
        offset: usize,
        #[serde(default = "default_limit")]
        limit: usize,
        filter: Option<String>,
    },
    /// Récupérer le détail d'une requête
    GetRequest { id: String },
    /// Effacer toutes les requêtes
    ClearRequests,
    /// Sauvegarder la session
    SaveSession { name: Option<String> },
    /// Export HAR
    ExportHar { name: Option<String> },
    /// État actuel du proxy
    Status,
    /// S'abonner aux événements temps réel
    Subscribe,
}

fn default_limit() -> usize {
    100
}

/// Réponses du moteur vers les UIs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApiResponse {
    Ok { message: String },
    Error { message: String },
    Status {
        paused: bool,
        port: u16,
        request_count: usize,
        ca_path: String,
    },
    Requests {
        items: Vec<crate::InterceptedRequest>,
        total: usize,
    },
    RequestDetail {
        request: crate::InterceptedRequest,
    },
    Saved { path: String },
    /// Message de bienvenue à la connexion
    Welcome {
        version: String,
        proxy_port: u16,
        api_port: u16,
    },
}
