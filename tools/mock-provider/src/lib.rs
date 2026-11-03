//! Simulateur de fournisseur de buNtu (T0.3).
//!
//! Il imite les endpoints OpenAI (`/v1/chat/completions`, `/v1/embeddings`) et Anthropic
//! (`/v1/messages`), avec un comportement piloté par des en-têtes `x-mock-*`.
//! Les réponses sont préconstruites au démarrage (`Bytes`) : le simulateur ne doit jamais
//! être le goulot des tests ni des benchmarks.

pub mod handlers;
pub mod payloads;

use std::sync::Arc;

use axum::Router;
use axum::routing::post;

use crate::handlers::AppState;
use crate::payloads::Payloads;

/// Construit le routeur complet, avec ses réponses préconstruites.
pub fn app() -> Router {
    let state = AppState {
        payloads: Arc::new(Payloads::build()),
    };

    Router::new()
        .route("/v1/chat/completions", post(handlers::chat_completions))
        .route("/v1/embeddings", post(handlers::embeddings))
        .route("/v1/messages", post(handlers::messages))
        .with_state(state)
}
