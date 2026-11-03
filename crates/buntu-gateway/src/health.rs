//! Routes de santé : `/health` (vivant) et `/ready` (prêt).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::extract::State;
use axum::http::StatusCode;

/// État de préparation partagé entre le serveur et la route `/ready`.
///
/// Il sera branché sur la configuration chargée en T1.2 ; en attendant, il vaut `true` dès que
/// le serveur écoute, et repasse à `false` au début de l'arrêt gracieux.
#[derive(Clone, Default)]
pub struct Readiness {
    ready: Arc<AtomicBool>,
}

impl Readiness {
    pub fn new(ready: bool) -> Self {
        Self {
            ready: Arc::new(AtomicBool::new(ready)),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }

    pub fn set(&self, ready: bool) {
        self.ready.store(ready, Ordering::Release);
    }
}

/// `/health` répond toujours 200 : il ne vérifie rien.
pub async fn health() -> StatusCode {
    StatusCode::OK
}

/// `/ready` répond 200 seulement quand la passerelle est prête à servir.
pub async fn ready(State(readiness): State<Readiness>) -> StatusCode {
    if readiness.is_ready() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}
