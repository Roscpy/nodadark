// nodadark-tui/src/state/mod.rs -> src/state.rs

use nodadark_engine::{EngineEvent, InterceptedRequest};
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq)]
pub enum ActivePanel {
    RequestList,
    RequestDetail,
    Search,
    PopupAction,
    PopupCookieEditor,
    PopupConfirmReplay,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DetailTab {
    Headers,
    Body,
    Hex,
}

pub struct AppState {
    // ── Liste des requêtes ──────────────────────────────────
    pub requests: VecDeque<InterceptedRequest>,
    pub list_offset: usize,       // Index de la sélection dans la liste
    pub list_scroll: usize,       // Scroll dans la liste (premier item visible)

    // ── Détail ──────────────────────────────────────────────
    pub selected_request: Option<InterceptedRequest>,
    pub detail_tab: DetailTab,
    pub detail_scroll: usize,

    // ── Navigation ──────────────────────────────────────────
    pub active_panel: ActivePanel,

    // ── Filtre de recherche ─────────────────────────────────
    pub search_input: String,
    pub search_active: bool,
    pub filter_text: String,      // Filtre actif (validé)

    // ── Proxy état ──────────────────────────────────────────
    pub proxy_paused: bool,
    pub proxy_port: u16,
    pub engine_connected: bool,

    // ── Messages de statut ──────────────────────────────────
    pub status_message: Option<String>,

    // ── Popup actions ───────────────────────────────────────
    pub popup_items: Vec<String>,
    pub popup_selected: usize,

    // ── Cookie editor ───────────────────────────────────────
    pub cookie_rows: Vec<(String, String)>,
    pub cookie_selected: usize,
    pub cookie_editing: Option<(usize, String)>, // (colonne 0=name/1=value, buffer)
}

impl AppState {
    pub fn new() -> Self {
        Self {
            requests: VecDeque::new(),
            list_offset: 0,
            list_scroll: 0,
            selected_request: None,
            detail_tab: DetailTab::Headers,
            detail_scroll: 0,
            active_panel: ActivePanel::RequestList,
            search_input: String::new(),
            search_active: false,
            filter_text: String::new(),
            proxy_paused: false,
            proxy_port: 8080,
            engine_connected: false,
            status_message: None,
            popup_items: vec![],
            popup_selected: 0,
            cookie_rows: vec![],
            cookie_selected: 0,
            cookie_editing: None,
        }
    }

    pub fn is_editing(&self) -> bool {
        self.search_active
            || self.cookie_editing.is_some()
            || self.active_panel == ActivePanel::PopupCookieEditor
    }

    /// Ajoute ou met à jour une requête
    pub fn upsert_request(&mut self, req: InterceptedRequest) {
        if let Some(pos) = self.requests.iter().position(|r| r.id == req.id) {
            self.requests[pos] = req.clone();
            // Rafraîchir la sélection si c'est la requête courante
            if let Some(sel) = &self.selected_request {
                if sel.id == req.id {
                    self.selected_request = Some(req);
                }
            }
        } else {
            self.requests.push_back(req);
            if self.requests.len() > 10_000 {
                self.requests.pop_front();
                if self.list_offset > 0 {
                    self.list_offset -= 1;
                }
            }
        }
    }

    pub fn handle_engine_event(&mut self, event: EngineEvent) {
        match event {
            EngineEvent::Request { id, method, url, host, timestamp, tls } => {
                let req = InterceptedRequest {
                    id,
                    method,
                    url,
                    host,
                    path: String::new(),
                    http_version: "HTTP/1.1".into(),
                    request_headers: vec![],
                    request_body: None,
                    response_status: None,
                    response_headers: vec![],
                    response_body: None,
                    duration_ms: None,
                    timestamp,
                    state: nodadark_engine::RequestState::Pending,
                    tls,
                    error: None,
                };
                self.upsert_request(req);
            }
            EngineEvent::Response { id, status, duration_ms, size } => {
                if let Some(pos) = self.requests.iter().position(|r| r.id == id) {
                    self.requests[pos].response_status = Some(status);
                    self.requests[pos].duration_ms = Some(duration_ms);
                    self.requests[pos].state = nodadark_engine::RequestState::Complete;
                }
            }
            EngineEvent::Dropped { id } => {
                if let Some(pos) = self.requests.iter().position(|r| r.id == id) {
                    self.requests[pos].state = nodadark_engine::RequestState::Dropped;
                }
            }
            EngineEvent::ProxyState { paused, port } => {
                self.proxy_paused = paused;
                self.proxy_port = port;
                self.status_message = Some(if paused {
                    "⏸  Proxy en pause".into()
                } else {
                    "▶  Proxy actif".into()
                });
            }
            _ => {}
        }
    }

    /// Requêtes filtrées selon le filtre actif
    pub fn filtered_requests(&self) -> Vec<&InterceptedRequest> {
        if self.filter_text.is_empty() {
            return self.requests.iter().collect();
        }
        let f = self.filter_text.to_lowercase();
        self.requests
            .iter()
            .filter(|r| {
                r.url.to_lowercase().contains(&f)
                    || r.host.to_lowercase().contains(&f)
                    || r.method.to_lowercase().contains(&f)
                    || r.response_status
                        .map(|s| s.to_string().contains(&f))
                        .unwrap_or(false)
            })
            .collect()
    }

    pub fn select_up(&mut self) {
        if self.list_offset > 0 {
            self.list_offset -= 1;
        }
    }

    pub fn select_down(&mut self) {
        let count = self.filtered_requests().len();
        if self.list_offset + 1 < count {
            self.list_offset += 1;
        }
    }

    pub fn get_selected(&self) -> Option<&InterceptedRequest> {
        self.filtered_requests().get(self.list_offset).copied()
    }

    pub fn open_detail(&mut self) {
        if let Some(req) = self.get_selected().cloned() {
            self.selected_request = Some(req);
            self.active_panel = ActivePanel::RequestDetail;
            self.detail_scroll = 0;
        }
    }

    pub fn open_action_popup(&mut self) {
        self.popup_items = vec![
            "↪  Replay (renvoyer tel quel)".into(),
            "✏  Éditer et rejouer".into(),
            "🍪  Éditer les cookies".into(),
            "✂  Dropper cette requête".into(),
            "📋  Copier l'URL".into(),
        ];
        self.popup_selected = 0;
        self.active_panel = ActivePanel::PopupAction;
    }

    pub fn open_cookie_editor(&mut self) {
        if let Some(req) = &self.selected_request {
            // Parser les cookies depuis les headers
            self.cookie_rows = parse_cookies_from_headers(&req.request_headers);
            self.cookie_selected = 0;
            self.active_panel = ActivePanel::PopupCookieEditor;
        }
    }
}

fn parse_cookies_from_headers(headers: &[(String, String)]) -> Vec<(String, String)> {
    for (name, value) in headers {
        if name.to_lowercase() == "cookie" {
            return value
                .split(';')
                .filter_map(|pair| {
                    let mut parts = pair.trim().splitn(2, '=');
                    let k = parts.next()?.trim().to_string();
                    let v = parts.next().unwrap_or("").trim().to_string();
                    Some((k, v))
                })
                .collect();
        }
    }
    vec![]
}
