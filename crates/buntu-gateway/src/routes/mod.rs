//! Routes de la passerelle et ce qu'elles partagent : en-têtes de provenance, réponses SSE,
//! horodatage et journal des requêtes.

pub mod anthropic;
pub mod openai;

use std::time::Instant;

use axum::response::{IntoResponse, Response};
use buntu_core::routing::Target;
use buntu_schema::{GatewayError, Usage};

use crate::state::GatewayState;

/// Ajoute les en-têtes de provenance `x-buntu-*` à une réponse.
pub fn add_routed_headers(response: &mut Response, provider: &str, model: &str, attempts: u32) {
    let headers = response.headers_mut();
    if let Ok(value) = axum::http::HeaderValue::from_str(provider) {
        headers.insert("x-buntu-provider", value);
    }
    if let Ok(value) = axum::http::HeaderValue::from_str(model) {
        headers.insert("x-buntu-model", value);
    }
    if let Ok(value) = axum::http::HeaderValue::from_str(&attempts.to_string()) {
        headers.insert("x-buntu-attempts", value);
    }
}

/// Réponse SSE : chaque morceau part dès qu'il est produit (jamais tamponné en entier).
/// `body` vient de `Body::from_stream` sur le flux de la route.
pub fn sse_response(body: axum::body::Body, target: &Target) -> Response {
    let mut response = body.into_response();
    let headers = response.headers_mut();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("text/event-stream"),
    );
    headers.insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-cache"),
    );
    add_routed_headers(&mut response, &target.provider, &target.model, 1);
    response
}

/// Secondes écoulées depuis l'époque Unix.
pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// Une requête en cours, journalisée une fois à sa sortie : succès comme erreur, JSON comme
/// flux (T1.18 : `try_log`, jamais bloquant).
#[derive(Clone)]
pub struct Journal {
    state: GatewayState,
    /// Identifiant de la requête, aussi utilisé comme identifiant de contexte.
    pub id: String,
    requested: String,
    started: Instant,
}

impl Journal {
    pub fn start(state: &GatewayState, body: &serde_json::Value) -> Self {
        Self {
            state: state.clone(),
            id: state.request_id(),
            requested: body
                .get("model")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            started: Instant::now(),
        }
    }

    pub fn success(&self, provider: &str, model: &str, attempts: u32, usage: &Usage) {
        self.record(provider, model, 200, attempts, usage);
    }

    /// Erreur avant ou pendant la réponse ; la cible n'est pas toujours connue.
    pub fn failure(&self, target: Option<&Target>, error: &GatewayError) {
        let (provider, model) = match target {
            Some(target) => (target.provider.as_str(), target.model.as_str()),
            None => ("", self.requested.as_str()),
        };
        self.record(provider, model, error.http_status(), 1, &Usage::default());
    }

    fn record(&self, provider: &str, model: &str, status: u16, attempts: u32, usage: &Usage) {
        self.state.record_request(buntu_store::RequestLogRecord {
            id: self.id.clone(),
            model: model.to_owned(),
            provider: provider.to_owned(),
            status,
            latency_ms: u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX),
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            created_at: unix_now(),
            attempts,
            rowid: 0,
        });
    }
}
