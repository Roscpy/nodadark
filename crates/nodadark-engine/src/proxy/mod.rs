// nodadark-engine/src/proxy/mod.rs

pub mod cert;
pub mod server;

use crate::{InterceptedRequest, RequestState};
use dashmap::DashMap;
use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

/// État global partagé entre le proxy et l'API
pub struct ProxyState {
    /// Dictionnaire des requêtes en mémoire (id → requête)
    pub requests: DashMap<String, InterceptedRequest>,
    /// File ordonnée des IDs (pour conserver l'ordre chronologique)
    pub request_order: Mutex<VecDeque<String>>,
    /// Proxy en pause ?
    pub paused: AtomicBool,
    /// Limite de requêtes en mémoire
    pub max_requests: usize,
    /// Mode Fail-Open
    pub fail_open: bool,
}

impl ProxyState {
    pub fn new(max_requests: usize, fail_open: bool) -> Self {
        Self {
            requests: DashMap::new(),
            request_order: Mutex::new(VecDeque::new()),
            paused: AtomicBool::new(false),
            max_requests,
            fail_open,
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }

    /// Insère ou met à jour une requête
    pub fn upsert(&self, req: InterceptedRequest) {
        let id = req.id.clone();
        let already_exists = self.requests.contains_key(&id);
        self.requests.insert(id.clone(), req);

        if !already_exists {
            let mut order = self.request_order.lock().unwrap();
            order.push_back(id);
            // Tronquer si on dépasse la limite
            while order.len() > self.max_requests {
                if let Some(old_id) = order.pop_front() {
                    self.requests.remove(&old_id);
                }
            }
        }
    }

    pub fn get(&self, id: &str) -> Option<InterceptedRequest> {
        self.requests.get(id).map(|r| r.clone())
    }

    pub fn drop_request(&self, id: &str) -> bool {
        if let Some(mut req) = self.requests.get_mut(id) {
            req.state = RequestState::Dropped;
            return true;
        }
        false
    }

    /// Retourne la liste paginée des requêtes (ordre chronologique)
    pub fn list(&self, offset: usize, limit: usize) -> Vec<InterceptedRequest> {
        let order = self.request_order.lock().unwrap();
        order
            .iter()
            .skip(offset)
            .take(limit)
            .filter_map(|id| self.requests.get(id).map(|r| r.clone()))
            .collect()
    }

    pub fn count(&self) -> usize {
        self.requests.len()
    }

    pub fn clear(&self) {
        self.requests.clear();
        self.request_order.lock().unwrap().clear();
    }
}
