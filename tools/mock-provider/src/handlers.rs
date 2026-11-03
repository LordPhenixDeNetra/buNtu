//! Gestionnaires HTTP et en-têtes de contrôle `x-mock-*`.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::{Json, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use serde::Deserialize;

use crate::payloads::Payloads;

const LATENCY_MS: &str = "x-mock-latency-ms";
const TOKENS: &str = "x-mock-tokens";
const TOKEN_INTERVAL_MS: &str = "x-mock-token-interval-ms";
const STATUS: &str = "x-mock-status";
const DROP_AFTER_TOKENS: &str = "x-mock-drop-after-tokens";

const DEFAULT_TOKENS: usize = 4;

#[derive(Clone)]
pub struct AppState {
    pub payloads: Arc<Payloads>,
}

/// Corps de requête commun : seul le drapeau `stream` nous intéresse.
#[derive(Deserialize)]
pub struct StreamRequest {
    #[serde(default)]
    pub stream: bool,
}

pub async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<StreamRequest>,
) -> Response {
    wait_latency(&headers).await;
    if let Some(status) = status_header(&headers) {
        return error_response(&state, status);
    }

    if request.stream {
        let body = chat_stream(
            Arc::clone(&state.payloads),
            token_count(&headers),
            token_interval(&headers),
            drop_after(&headers),
        );
        sse_response(body)
    } else {
        json_response(state.payloads.chat_json.clone())
    }
}

pub async fn embeddings(State(state): State<AppState>, headers: HeaderMap) -> Response {
    wait_latency(&headers).await;
    if let Some(status) = status_header(&headers) {
        return error_response(&state, status);
    }
    json_response(state.payloads.embeddings_json.clone())
}

pub async fn messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<StreamRequest>,
) -> Response {
    wait_latency(&headers).await;
    if let Some(status) = status_header(&headers) {
        return error_response(&state, status);
    }

    if request.stream {
        let body = anthropic_stream(
            Arc::clone(&state.payloads),
            token_count(&headers),
            token_interval(&headers),
            drop_after(&headers),
        );
        sse_response(body)
    } else {
        json_response(state.payloads.anthropic_json.clone())
    }
}

fn number(headers: &HeaderMap, name: &str) -> Option<u64> {
    headers.get(name)?.to_str().ok()?.parse().ok()
}

fn token_count(headers: &HeaderMap) -> usize {
    number(headers, TOKENS).map_or(DEFAULT_TOKENS, |value| value as usize)
}

fn token_interval(headers: &HeaderMap) -> Duration {
    Duration::from_millis(number(headers, TOKEN_INTERVAL_MS).unwrap_or(0))
}

fn drop_after(headers: &HeaderMap) -> Option<usize> {
    number(headers, DROP_AFTER_TOKENS).map(|value| value as usize)
}

fn status_header(headers: &HeaderMap) -> Option<u16> {
    number(headers, STATUS).map(|value| value as u16)
}

async fn wait_latency(headers: &HeaderMap) {
    match number(headers, LATENCY_MS) {
        Some(0) | None => {}
        Some(milliseconds) => tokio::time::sleep(Duration::from_millis(milliseconds)).await,
    }
}

fn json_response(body: Bytes) -> Response {
    ([(header::CONTENT_TYPE, "application/json")], body).into_response()
}

fn sse_response(body: Body) -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/event-stream"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        body,
    )
        .into_response()
}

fn error_response(state: &AppState, status: u16) -> Response {
    let status = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let mut response = (status, state.payloads.error_json.clone()).into_response();
    if status == StatusCode::TOO_MANY_REQUESTS {
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, HeaderValue::from_static("1"));
    }
    response
}

fn drop_error() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "mock drop")
}

/// Flux SSE au format OpenAI, avec latence, intervalle et coupure optionnels.
fn chat_stream(
    payloads: Arc<Payloads>,
    tokens: usize,
    interval: Duration,
    drop_after: Option<usize>,
) -> Body {
    Body::from_stream(async_stream::stream! {
        for index in 0..tokens {
            if drop_after == Some(index) {
                yield Err(drop_error());
                return;
            }
            if !interval.is_zero() {
                tokio::time::sleep(interval).await;
            }
            yield Ok(payloads.chat_chunk.clone());
        }
        yield Ok(payloads.chat_final.clone());
        yield Ok(payloads.chat_done.clone());
    })
}

/// Flux SSE au format Anthropic (événements nommés).
fn anthropic_stream(
    payloads: Arc<Payloads>,
    tokens: usize,
    interval: Duration,
    drop_after: Option<usize>,
) -> Body {
    Body::from_stream(async_stream::stream! {
        yield Ok(payloads.anthropic_start.clone());
        yield Ok(payloads.anthropic_block_start.clone());
        for index in 0..tokens {
            if drop_after == Some(index) {
                yield Err(drop_error());
                return;
            }
            if !interval.is_zero() {
                tokio::time::sleep(interval).await;
            }
            yield Ok(payloads.anthropic_delta.clone());
        }
        yield Ok(payloads.anthropic_block_stop.clone());
        yield Ok(payloads.anthropic_message_delta.clone());
        yield Ok(payloads.anthropic_stop.clone());
    })
}
