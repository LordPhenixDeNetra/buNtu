//! Réponses d'erreur normalisées (T1.15).
//!
//! Format OpenAI `{"error":{"message","type","code"}}`, statut HTTP du [`GatewayError`], et
//! en-tête `x-buntu-error-source` (`client`, `provider`, `budget`, `gateway`).

use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use buntu_schema::GatewayError;
use serde_json::json;

/// Convertit une erreur canonique en réponse HTTP normalisée.
pub fn response(error: &GatewayError) -> Response {
    let status =
        StatusCode::from_u16(error.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let body = json!({
        "error": {
            "message": error.to_string(),
            "type": error.error_type(),
            "code": error.code(),
        }
    });
    let mut response = (status, Json(body)).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    if let Ok(source) = HeaderValue::from_str(error.source()) {
        response
            .headers_mut()
            .insert("x-buntu-error-source", source);
    }
    if let Some(retry_after) = error.retry_after()
        && let Ok(value) = HeaderValue::from_str(&retry_after.as_secs().to_string())
    {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    response
}
