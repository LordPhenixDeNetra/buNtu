//! Routes de la passerelle.

pub mod anthropic;
pub mod openai;

use axum::response::Response;

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
