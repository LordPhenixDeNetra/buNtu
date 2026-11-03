//! Passerelle HTTP de buNtu : routes de santé et serveur.
//!
//! Les routes métier (`/v1/...`) arriveront avec T1.13 ; ce crate expose pour l'instant le
//! serveur, `/health`, `/ready` et les limites de base (corps, en-têtes, connexions).

pub mod health;
pub mod server;
pub mod shutdown;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::get;
use tower_http::limit::RequestBodyLimitLayer;

pub use health::Readiness;
pub use server::{ServerConfig, serve};

/// Taille maximale du corps d'une requête : 10 Mo (cahier §8.1).
pub const MAX_BODY_BYTES: usize = 10 * 1024 * 1024;

/// Construit le routeur de la passerelle.
pub fn app(readiness: Readiness) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/ready", get(health::ready))
        .with_state(readiness)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(RequestBodyLimitLayer::new(MAX_BODY_BYTES))
}
