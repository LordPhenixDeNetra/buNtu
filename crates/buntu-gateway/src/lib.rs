//! Passerelle HTTP de buNtu : routes de santé, endpoints compatibles OpenAI et serveur.

pub mod admin;
pub mod error_response;
pub mod executor;
pub mod health;
pub mod registry;
pub mod routes;
pub mod server;
pub mod shutdown;
pub mod state;
pub mod ui;
pub mod upstream;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use tower_http::limit::RequestBodyLimitLayer;

pub use error_response::response as error_response;
pub use health::Readiness;
pub use server::{ServerConfig, serve};
pub use state::GatewayState;

/// Taille maximale du corps d'une requête : 10 Mo (cahier §8.1).
pub const MAX_BODY_BYTES: usize = 10 * 1024 * 1024;

/// Routeur de santé seul (tests historiques, sans configuration).
pub fn app(readiness: Readiness) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/ready", get(health::ready))
        .with_state(readiness)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(RequestBodyLimitLayer::new(MAX_BODY_BYTES))
}

/// Exposition Prometheus (T1.19) : 503 tant que l'exportateur n'est pas installé.
async fn metrics_endpoint() -> axum::response::Response {
    match buntu_observe::metrics::render_global() {
        Some(body) => (
            [(
                axum::http::header::CONTENT_TYPE,
                "text/plain; version=0.0.4; charset=utf-8",
            )],
            body,
        )
            .into_response(),
        None => axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

/// Routeur complet : santé + endpoints OpenAI (T1.13).
pub fn app_with_state(state: GatewayState) -> Router {
    let health_router = Router::new()
        .route("/health", get(health::health))
        .route("/ready", get(health::ready))
        .with_state(state.readiness.clone());
    let api_router = Router::new()
        .route(
            "/v1/chat/completions",
            post(routes::openai::chat::chat_completions),
        )
        .route(
            "/v1/embeddings",
            post(routes::openai::embeddings::embeddings),
        )
        .route("/v1/models", get(routes::openai::models::models))
        .route("/metrics", get(metrics_endpoint))
        .route(
            "/anthropic/v1/messages",
            post(routes::anthropic::messages::messages),
        )
        .route(
            "/admin/v1/providers",
            get(admin::handlers::list_providers).post(admin::handlers::create_provider),
        )
        .route(
            "/admin/v1/providers/{id}",
            get(admin::handlers::get_provider)
                .patch(admin::handlers::update_provider)
                .delete(admin::handlers::delete_provider),
        )
        .route(
            "/admin/v1/providers/{id}/keys",
            post(admin::handlers::create_key),
        )
        .route(
            "/admin/v1/providers/{id}/keys/{index}",
            delete(admin::handlers::delete_key),
        )
        .route(
            "/admin/v1/routes",
            get(admin::handlers::list_routes).post(admin::handlers::create_route),
        )
        .route(
            "/admin/v1/routes/{alias}",
            get(admin::handlers::get_route)
                .patch(admin::handlers::update_route)
                .delete(admin::handlers::delete_route),
        )
        .route("/admin/v1/openapi.json", get(admin::handlers::openapi_json))
        .route("/admin/v1/requests", get(admin::handlers::list_requests))
        .route("/admin/v1/requests/{id}", get(admin::handlers::get_request))
        .route("/", get(ui::index))
        .route("/{*path}", get(ui::asset))
        .with_state(state);
    health_router
        .merge(api_router)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(RequestBodyLimitLayer::new(MAX_BODY_BYTES))
}
