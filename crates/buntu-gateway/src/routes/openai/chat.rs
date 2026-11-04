//! Endpoint `/v1/chat/completions` (T1.13) : JSON et flux SSE.
//!
//! Le flux respecte le piège de la fiche : `Content-Type: text/event-stream` et chaque morceau
//! est écrit immédiatement (aucun tamponnage complet), avec `data: [DONE]` final.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response as AxumResponse};
use buntu_core::context::RequestContext;
use buntu_core::fallback::FallbackChain;
use buntu_core::routing;
use buntu_providers::provider::StreamState;
use buntu_schema::GatewayError;
use bytes::Bytes;
use http_body_util::BodyExt;
use serde_json::Value;

use crate::error_response;
use crate::executor::HttpExecutor;
use crate::state::GatewayState;

use super::request;
use super::response as wire;

pub async fn chat_completions(
    State(state): State<GatewayState>,
    Json(body): Json<Value>,
) -> AxumResponse {
    match run(&state, body).await {
        Ok(response) => response,
        Err(error) => error_response::response(&error),
    }
}

async fn run(state: &GatewayState, body: Value) -> Result<AxumResponse, GatewayError> {
    let request = request::parse(&body)?;
    let config = state.config.snapshot();
    let targets = routing::resolve(
        &config,
        &request.model,
        routing::required_capabilities(&request),
    )?;
    let context = RequestContext::new(state.request_id());
    let executor = HttpExecutor::from_state(state);

    if request.stream {
        return stream(&executor, &targets, &request, &context).await;
    }

    let started = std::time::Instant::now();
    let routed = FallbackChain::default()
        .execute(&targets, &executor, &context, &request)
        .await?;
    let latency = started.elapsed();
    // Journal des requêtes (T1.24) : `try_log` non bloquant (T1.18).
    state.record_request(buntu_store::RequestLogRecord {
        id: context.id.clone(),
        model: routed.model.clone(),
        provider: routed.provider.clone(),
        status: 200,
        latency_ms: u64::try_from(latency.as_millis()).unwrap_or(u64::MAX),
        input_tokens: routed.response.usage.input_tokens,
        output_tokens: routed.response.usage.output_tokens,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or(0),
        attempts: routed.attempts,
        rowid: 0,
    });
    let mut response = Json(wire::to_openai(&routed.response)).into_response();
    crate::routes::add_routed_headers(
        &mut response,
        &routed.provider,
        &routed.model,
        routed.attempts,
    );
    Ok(response)
}

async fn stream(
    executor: &HttpExecutor,
    targets: &[buntu_core::routing::Target],
    request: &buntu_schema::Request,
    context: &RequestContext,
) -> Result<AxumResponse, GatewayError> {
    let mut opened = None;
    let mut last_error = None;
    // Repli avant le premier octet uniquement : une fois le flux commencé, aucune reprise.
    for target in targets {
        match executor.open_stream(target, request).await {
            Ok((provider, _status, _headers, body)) => {
                opened = Some((provider, target.clone(), body));
                break;
            }
            Err(error) => last_error = Some(error),
        }
    }
    let Some((provider, target, body)) = opened else {
        return Err(last_error.unwrap_or_else(|| GatewayError::Internal {
            message: "aucune cible pour le flux".to_owned(),
        }));
    };

    let id = format!("chatcmpl-{}", context.id);
    let model = target.model.clone();
    let stream = async_stream::stream! {
        let mut stream_state = StreamState::new();
        let mut body = body;
        while let Some(Ok(frame)) = body.frame().await {
            let Ok(data) = frame.into_data() else {
                continue;
            };
            match provider.parse_stream_chunk(&mut stream_state, data) {
                Ok(events) => {
                    for event in &events {
                        if let Some(chunk) = wire::chunk(event, &id, &model) {
                            yield Ok::<Bytes, std::convert::Infallible>(
                                Bytes::from(format!("data: {chunk}\n\n")),
                            );
                        }
                    }
                }
                Err(error) => {
                    let frame = serde_json::json!({
                        "error": { "message": error.to_string(), "type": error.error_type(), "code": error.code() }
                    });
                    yield Ok(Bytes::from(format!("data: {frame}\n\n")));
                    break;
                }
            }
        }
        yield Ok(Bytes::from_static(b"data: [DONE]\n\n"));
    };

    let mut response = axum::body::Body::from_stream(stream).into_response();
    let headers = response.headers_mut();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("text/event-stream"),
    );
    headers.insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-cache"),
    );
    crate::routes::add_routed_headers(&mut response, &target.provider, &target.model, 1);
    Ok(response)
}
