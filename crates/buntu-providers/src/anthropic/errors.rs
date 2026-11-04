//! Classification des erreurs HTTP d'Anthropic.
//!
//! Référence : <https://docs.anthropic.com/en/api/errors> (consultée le 2026-11-04).

use buntu_schema::GatewayError;
use hyper::HeaderMap;
use serde_json::Value;

pub fn map(status: u16, headers: &HeaderMap, body: &[u8]) -> GatewayError {
    let message = message_of(body);
    match status {
        400 | 404 | 422 => GatewayError::InvalidRequest { message },
        401 | 403 => GatewayError::Authentication,
        429 => GatewayError::ProviderRateLimited {
            retry_after: retry_after(headers),
        },
        500 | 502 | 503 | 504 | 529 => GatewayError::ProviderUnavailable { message },
        other => GatewayError::ProviderUnavailable {
            message: format!("statut {other} : {message}"),
        },
    }
}

fn retry_after(headers: &HeaderMap) -> Option<std::time::Duration> {
    headers
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .and_then(|text| text.parse::<u64>().ok())
        .map(std::time::Duration::from_secs)
}

fn message_of(body: &[u8]) -> String {
    let Ok(value) = serde_json::from_slice::<Value>(body) else {
        return String::from_utf8_lossy(body).chars().take(200).collect();
    };
    value
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .map(|message| message.chars().take(200).collect())
        .unwrap_or_else(|| "erreur Anthropic sans message".to_owned())
}
