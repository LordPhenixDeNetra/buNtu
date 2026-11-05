//! Endpoint `/v1/chat/completions` (T1.13) : JSON et flux SSE.
//!
//! Le flux respecte le piège de la fiche : `Content-Type: text/event-stream` et chaque morceau
//! est écrit immédiatement (aucun tamponnage complet), avec `data: [DONE]` final.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response as AxumResponse};
use buntu_core::context::RequestContext;
use buntu_core::routing;
use buntu_core::timeouts::{TimedExecutor, Timeouts};
use buntu_providers::provider::StreamState;
use buntu_schema::{GatewayError, StreamEvent, Usage};
use bytes::Bytes;
use serde_json::Value;

use crate::error_response;
use crate::executor::HttpExecutor;
use crate::routes::{Journal, add_routed_headers, sse_response};
use crate::state::GatewayState;

use super::request;
use super::response as wire;

pub async fn chat_completions(
    State(state): State<GatewayState>,
    Json(body): Json<Value>,
) -> AxumResponse {
    let journal = Journal::start(&state, &body);
    match run(&state, body, &journal).await {
        Ok(response) => response,
        Err(error) => {
            journal.failure(None, &error);
            error_response::response(&error)
        }
    }
}

async fn run(
    state: &GatewayState,
    body: Value,
    journal: &Journal,
) -> Result<AxumResponse, GatewayError> {
    let request = request::parse(&body)?;
    let config = state.config.snapshot();
    let targets = routing::resolve(
        &config,
        &request.model,
        routing::required_capabilities(&request),
    )?;
    let context = RequestContext::new(journal.id.clone());
    let executor = HttpExecutor::from_state(state);
    let timeouts = executor.timeouts_for(&request.model);

    if request.stream {
        return stream(&executor, &targets, &request, journal, timeouts).await;
    }

    let routed = state
        .fallback
        .execute(
            &targets,
            &TimedExecutor::new(&executor, timeouts),
            &context,
            &request,
        )
        .await?;
    journal.success(
        &routed.provider,
        &routed.model,
        routed.attempts,
        &routed.response.usage,
    );
    let mut response = Json(wire::to_openai(&routed.response)).into_response();
    add_routed_headers(
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
    journal: &Journal,
    timeouts: Timeouts,
) -> Result<AxumResponse, GatewayError> {
    let opened = executor
        .open_first_stream(targets, request, timeouts)
        .await?;
    let (provider, target, mut chunks) = (opened.provider, opened.target, opened.chunks);

    let id = format!("chatcmpl-{}", journal.id);
    let model = target.model.clone();
    let journal = journal.clone();
    let logged = target.clone();
    let stream = async_stream::stream! {
        let mut stream_state = StreamState::new();
        let mut usage = Usage::default();
        let mut failed = None;
        while let Some(next) = chunks.next().await {
            match next.and_then(|data| provider.parse_stream_chunk(&mut stream_state, data)) {
                Ok(events) => {
                    for event in &events {
                        if let StreamEvent::Usage { usage: counted } = event {
                            usage = *counted;
                        }
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
                    failed = Some(error);
                    break;
                }
            }
        }
        yield Ok(Bytes::from_static(b"data: [DONE]\n\n"));
        match failed {
            Some(error) => journal.failure(Some(&logged), &error),
            None => journal.success(&logged.provider, &logged.model, 1, &usage),
        }
    };

    Ok(sse_response(axum::body::Body::from_stream(stream), &target))
}
