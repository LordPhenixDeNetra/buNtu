//! Endpoint `/v1/embeddings` (T1.13).

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response as AxumResponse};
use buntu_core::routing;
use buntu_core::timeouts::with_timeout;
use buntu_schema::{EmbeddingsRequest, GatewayError};
use serde_json::{Map, Value};

use crate::error_response;
use crate::executor::HttpExecutor;
use crate::routes::Journal;
use crate::state::GatewayState;

use super::response as wire;

pub async fn embeddings(
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
    let request = parse_embeddings(&body)?;
    let config = state.config.snapshot();
    let targets = routing::resolve(&config, &request.model, routing::Capabilities::NONE)?;
    let executor = HttpExecutor::from_state(state);
    let timeouts = executor.timeouts_for(&request.model);
    // Réponse complète : même règle que `TimedExecutor` (premier octet, sous plafond total).
    let deadline = timeouts.first_byte.min(timeouts.total);

    let mut last_error = None;
    for target in &targets {
        match with_timeout(deadline, executor.execute_embeddings(target, &request)).await {
            Ok(response) => {
                let usage = buntu_schema::Usage {
                    input_tokens: response.usage.prompt_tokens,
                    ..Default::default()
                };
                journal.success(&target.provider, &target.model, 1, &usage);
                let mut http = Json(wire::embeddings_to_openai(&response)).into_response();
                crate::routes::add_routed_headers(&mut http, &target.provider, &target.model, 1);
                return Ok(http);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| GatewayError::Internal {
        message: "aucune cible pour les embeddings".to_owned(),
    }))
}

fn parse_embeddings(body: &Value) -> Result<EmbeddingsRequest, GatewayError> {
    let model = body
        .get("model")
        .and_then(Value::as_str)
        .ok_or_else(|| GatewayError::InvalidRequest {
            message: "`model` est requis".to_owned(),
        })?
        .to_owned();
    let input = match body.get("input") {
        Some(Value::String(text)) => vec![text.clone()],
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| GatewayError::InvalidRequest {
                        message: "`input` doit contenir des chaînes".to_owned(),
                    })
            })
            .collect::<Result<Vec<String>, GatewayError>>()?,
        _ => {
            return Err(GatewayError::InvalidRequest {
                message: "`input` est requis".to_owned(),
            });
        }
    };
    let known = ["model", "input", "dimensions", "encoding_format"];
    let extra = body
        .as_object()
        .map(|object| {
            object
                .iter()
                .filter(|(key, _)| !known.contains(&key.as_str()))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect::<Map<String, Value>>()
        })
        .unwrap_or_default();
    Ok(EmbeddingsRequest {
        model,
        input,
        dimensions: body
            .get("dimensions")
            .and_then(Value::as_u64)
            .map(|value| value as u32),
        encoding_format: body
            .get("encoding_format")
            .and_then(Value::as_str)
            .map(str::to_owned),
        extra,
    })
}
