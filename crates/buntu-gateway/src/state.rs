//! État partagé de la passerelle : configuration, traducteurs, client amont et administration.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use buntu_core::config::ConfigHandle;
use buntu_observe::request_log::RequestLog;
use buntu_store::Store;

use crate::admin::AdminStore;
use crate::health::Readiness;
use crate::registry::ProviderRegistry;
use crate::upstream::UpstreamClient;

/// Tout ce dont les routes ont besoin, clonable et partageable.
#[derive(Clone)]
pub struct GatewayState {
    pub readiness: Readiness,
    pub config: ConfigHandle,
    pub registry: Arc<ProviderRegistry>,
    pub upstream: UpstreamClient,
    pub admin: Arc<AdminStore>,
    /// Stockage partagé (journal des requêtes, T1.24).
    pub store: Option<Arc<Store>>,
    /// Journal asynchrone : `try_send`, jamais bloquant (T1.18).
    pub request_log: Option<RequestLog>,
    next_request_id: Arc<AtomicU64>,
}

impl GatewayState {
    pub fn new(readiness: Readiness, config: ConfigHandle) -> Self {
        Self {
            readiness,
            config,
            registry: Arc::new(ProviderRegistry::new()),
            upstream: UpstreamClient::new(),
            admin: Arc::new(AdminStore::empty()),
            store: None,
            request_log: None,
            next_request_id: Arc::new(AtomicU64::new(1)),
        }
    }

    pub fn with_registry(mut self, registry: Arc<ProviderRegistry>) -> Self {
        self.registry = registry;
        self
    }

    pub fn with_upstream(mut self, upstream: UpstreamClient) -> Self {
        self.upstream = upstream;
        self
    }

    pub fn with_admin(mut self, admin: Arc<AdminStore>) -> Self {
        self.admin = admin;
        self
    }

    /// Branche le stockage SQLite et démarre l'écriture asynchrone du journal.
    pub fn with_storage(mut self, store: Arc<Store>) -> Self {
        let (log, _task) = RequestLog::start((*store).clone());
        self.store = Some(store);
        self.request_log = Some(log);
        self
    }

    /// Enregistre une requête sans jamais bloquer le chemin chaud.
    pub fn record_request(&self, record: buntu_store::RequestLogRecord) {
        if let Some(log) = &self.request_log {
            let _ = log.try_log(record);
        }
    }

    /// Identifiant de requête, pour le contexte et les journaux.
    pub fn request_id(&self) -> String {
        let id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        format!("req-{id}")
    }
}
