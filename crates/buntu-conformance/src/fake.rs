//! Fournisseur factice du kit de conformité : traduction correcte, et variante cassée.

use buntu_providers::provider::{HttpRequest, Provider, StreamState};
use buntu_schema::{
    Capabilities, Choice, ContentPart, GatewayError, Message, Request, Response, Role, StopReason,
    StreamEvent, ToolCall, Usage,
};
use bytes::Bytes;
use hyper::HeaderMap;
use serde_json::{Map, Value};

const DEFAULT_ENDPOINT: &str = "/v1/fake/chat";

/// Fournisseur factice dont le protocole JSON est volontairement simple.
#[derive(Debug, Default, Clone)]
pub struct FakeProvider {
    pub endpoint: String,
}

impl FakeProvider {
    pub fn with_endpoint(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }

    fn endpoint(&self) -> &str {
        if self.endpoint.is_empty() {
            DEFAULT_ENDPOINT
        } else {
            &self.endpoint
        }
    }
}

fn internal(message: String) -> GatewayError {
    GatewayError::Internal { message }
}

fn stop_from_str(reason: &str) -> Result<StopReason, GatewayError> {
    serde_json::from_value::<StopReason>(Value::String(reason.to_owned()))
        .map_err(|error| internal(format!("raison d'arrêt invalide : {error}")))
}

fn parse_content(value: Option<&Value>) -> Result<Vec<ContentPart>, GatewayError> {
    let mut parts = Vec::new();
    let Some(items) = value.and_then(Value::as_array) else {
        return Ok(parts);
    };
    for item in items {
        match item.get("type").and_then(Value::as_str) {
            Some("text") => parts.push(ContentPart::Text {
                text: item
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            }),
            Some("tool_call") => parts.push(ContentPart::ToolCall(ToolCall {
                id: item
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                name: item
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                arguments: item.get("arguments").cloned().unwrap_or(Value::Null),
            })),
            other => {
                return Err(internal(format!("type de contenu inconnu : {other:?}")));
            }
        }
    }
    Ok(parts)
}

fn parse_usage(value: Option<&Value>) -> Usage {
    let Some(usage) = value else {
        return Usage::default();
    };
    Usage {
        input_tokens: usage
            .get("input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
        output_tokens: usage
            .get("output_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
        cached_input_tokens: usage
            .get("cached_input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
        reasoning_tokens: usage
            .get("reasoning_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
    }
}

impl Provider for FakeProvider {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn build_request(&self, request: &Request) -> Result<HttpRequest, GatewayError> {
        let body = serde_json::to_vec(request).map_err(|error| internal(error.to_string()))?;
        hyper::Request::builder()
            .method(hyper::Method::POST)
            .uri(self.endpoint())
            .header("content-type", "application/json")
            .header("authorization", "Bearer test")
            .body(Bytes::from(body))
            .map_err(|error| internal(error.to_string()))
    }

    fn parse_response(&self, body: Bytes) -> Result<Response, GatewayError> {
        let value: Value = serde_json::from_slice(&body)
            .map_err(|error| internal(format!("JSON invalide : {error}")))?;
        let choices = value
            .get("choices")
            .and_then(Value::as_array)
            .ok_or_else(|| internal("choices absent".to_owned()))?;
        let mut parsed = Vec::with_capacity(choices.len());
        let mut stop_reason = None;
        for (index, choice) in choices.iter().enumerate() {
            let message = choice
                .get("message")
                .ok_or_else(|| internal("message absent".to_owned()))?;
            let role = match message.get("role").and_then(Value::as_str) {
                Some("assistant") => Role::Assistant,
                Some("user") => Role::User,
                Some("system") => Role::System,
                Some("tool") => Role::Tool,
                other => return Err(internal(format!("rôle inconnu : {other:?}"))),
            };
            let content = parse_content(message.get("content"))?;
            if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
                stop_reason = Some(stop_from_str(reason)?);
            }
            parsed.push(Choice {
                index: index as u32,
                message: Message {
                    role,
                    content,
                    name: None,
                    extra: Map::new(),
                },
            });
        }

        Ok(Response {
            id: value
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            model: value
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            choices: parsed,
            usage: parse_usage(value.get("usage")),
            stop_reason,
            extra: Map::new(),
        })
    }

    fn parse_stream_chunk(
        &self,
        state: &mut StreamState,
        chunk: Bytes,
    ) -> Result<Vec<StreamEvent>, GatewayError> {
        state.push(&chunk);
        let mut events = Vec::new();
        for line in state.take_lines() {
            let text = std::str::from_utf8(&line)
                .map_err(|error| internal(format!("ligne non UTF-8 : {error}")))?;
            let Some(data) = text.strip_prefix("data: ") else {
                continue;
            };
            if data.trim() == "[DONE]" {
                state.mark_finished();
                events.push(StreamEvent::Done);
                continue;
            }
            let value: Value = serde_json::from_str(data)
                .map_err(|error| internal(format!("événement invalide : {error}")))?;
            match value.get("event").and_then(Value::as_str) {
                Some("start") => events.push(StreamEvent::Start {
                    id: value
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                    model: value
                        .get("model")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                }),
                Some("delta") => events.push(StreamEvent::ContentDelta {
                    index: value.get("index").and_then(Value::as_u64).unwrap_or(0) as u32,
                    part: ContentPart::Text {
                        text: value
                            .get("text")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                    },
                }),
                Some("stop") => events.push(StreamEvent::Stop {
                    reason: stop_from_str(
                        value
                            .get("reason")
                            .and_then(Value::as_str)
                            .unwrap_or("end_turn"),
                    )?,
                }),
                other => return Err(internal(format!("événement inconnu : {other:?}"))),
            }
        }
        state.record_events(events.len());
        Ok(events)
    }

    fn map_error(&self, status: u16, headers: &HeaderMap, _body: &[u8]) -> GatewayError {
        match status {
            400 | 422 => GatewayError::InvalidRequest {
                message: format!("statut {status}"),
            },
            401 | 403 => GatewayError::Authentication,
            429 => {
                let retry_after = headers
                    .get("retry-after")
                    .and_then(|value| value.to_str().ok())
                    .and_then(|text| text.parse::<u64>().ok())
                    .map(std::time::Duration::from_secs);
                GatewayError::ProviderRateLimited { retry_after }
            }
            _ => GatewayError::ProviderUnavailable {
                message: format!("statut {status}"),
            },
        }
    }

    fn capabilities(&self, _model: &str) -> Capabilities {
        Capabilities {
            tools: true,
            vision: true,
        }
    }
}

/// Fournisseur volontairement mal traduit : sert à prouver que la batterie détecte l'erreur.
pub struct BrokenProvider(pub FakeProvider);

impl BrokenProvider {
    pub fn new() -> Self {
        Self(FakeProvider::default())
    }
}

impl Default for BrokenProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Provider for BrokenProvider {
    fn name(&self) -> &'static str {
        "broken"
    }

    fn build_request(&self, request: &Request) -> Result<HttpRequest, GatewayError> {
        self.0.build_request(request)
    }

    fn parse_response(&self, body: Bytes) -> Result<Response, GatewayError> {
        let mut response = self.0.parse_response(body)?;
        response.stop_reason = Some(StopReason::MaxTokens);
        response.usage.output_tokens = response.usage.output_tokens.saturating_add(1);
        Ok(response)
    }

    fn parse_stream_chunk(
        &self,
        state: &mut StreamState,
        chunk: Bytes,
    ) -> Result<Vec<StreamEvent>, GatewayError> {
        self.0.parse_stream_chunk(state, chunk)
    }

    fn map_error(&self, status: u16, headers: &HeaderMap, body: &[u8]) -> GatewayError {
        self.0.map_error(status, headers, body)
    }

    fn capabilities(&self, model: &str) -> Capabilities {
        self.0.capabilities(model)
    }
}
