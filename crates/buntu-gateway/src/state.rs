//! État partagé de la passerelle : configuration, traducteurs, client amont et administration.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use buntu_core::config::ConfigHandle;

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

    /// Identifiant de requête, pour le contexte et les journaux.
    pub fn request_id(&self) -> String {
        let id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        format!("req-{id}")
    }
}
