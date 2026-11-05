//! État partagé de la passerelle : configuration, traducteurs, client amont et administration.

use std::sync::Arc;

use buntu_core::config::ConfigHandle;
use buntu_core::fallback::FallbackChain;
use buntu_observe::request_log::RequestLog;
use buntu_security::tokens::{hash_token, verify_token};
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
    /// Stockage partagé (journal des requêtes, T1.24 ; jetons admin).
    pub store: Option<Arc<Store>>,
    /// Journal asynchrone : `try_send`, jamais bloquant (T1.18).
    pub request_log: Option<RequestLog>,
    /// Chaîne de repli unique : son budget de réessais vaut pour **toutes** les requêtes (T1.5).
    pub fallback: Arc<FallbackChain>,
    /// Empreinte du jeton admin fourni au démarrage (`BUNTU_ADMIN_TOKEN`), jamais le clair.
    admin_token_hash: Option<String>,
}

impl GatewayState {
    pub fn new(readiness: Readiness, config: ConfigHandle) -> Self {
        Self {
            readiness,
            config,
            registry: Arc::new(ProviderRegistry::new()),
            upstream: UpstreamClient::new(),
            admin: Arc::new(AdminStore::default()),
            store: None,
            request_log: None,
            fallback: Arc::new(FallbackChain::default()),
            admin_token_hash: None,
        }
    }

    pub fn with_registry(mut self, registry: Arc<ProviderRegistry>) -> Self {
        self.registry = registry;
        self
    }

    /// Accepte ce jeton sur l'API d'administration (seule son empreinte est gardée).
    pub fn with_admin_token(mut self, token: &str) -> Self {
        self.admin_token_hash = Some(hash_token(token));
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

    /// Identifiant de requête, unique d'un démarrage à l'autre (clé primaire du journal).
    pub fn request_id(&self) -> String {
        format!("req-{:016x}{:016x}", fastrand::u64(..), fastrand::u64(..))
    }

    /// Jeton admin valide : celui du démarrage, ou un jeton actif (non révoqué) du stockage.
    /// Sans l'un ni l'autre, l'API d'administration reste fermée.
    pub async fn admin_token_is_valid(&self, token: &str) -> bool {
        if let Some(expected) = &self.admin_token_hash
            && verify_token(expected, token).unwrap_or(false)
        {
            return true;
        }
        match &self.store {
            Some(store) => store
                .api_token_is_active(&hash_token(token))
                .await
                .unwrap_or(false),
            None => false,
        }
    }
}
