//! Point d'entrée natif Anthropic `/anthropic/v1/messages` (T1.14).
//!
//! Références officielles consultées le 2026-11-04 :
//! - Messages : <https://docs.anthropic.com/en/api/messages>
//! - Streaming : <https://docs.anthropic.com/en/api/messages-streaming>
//!
//! L'en-tête `x-api-key` porte la clé virtuelle (gouvernance T1.20) ; il n'est pas transmis tel
//! quel au fournisseur (le client amont utilise la clé configurée). Le routage accepte
//! n'importe quel fournisseur : la réponse est toujours rendue au format Anthropic.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response as AxumResponse};
use buntu_core::context::RequestContext;
use buntu_core::routing;
use buntu_core::timeouts::{TimedExecutor, Timeouts};
use buntu_providers::provider::StreamState;
use buntu_schema::{
    ContentPart, GatewayError, ImageSource, Message, Request, Role, StopReason, Tool, ToolChoice,
};
use bytes::Bytes;
use serde_json::{Map, Value, json};

use crate::error_response;
use crate::executor::HttpExecutor;
use crate::state::GatewayState;

pub async fn messages(State(state): State<GatewayState>, Json(body): Json<Value>) -> AxumResponse {
    match run(&state, body).await {
        Ok(response) => response,
        Err(error) => error_response::response(&error),
    }
}

async fn run(state: &GatewayState, body: Value) -> Result<AxumResponse, GatewayError> {
    let request = parse_request(&body)?;
    let config = state.config.snapshot();
    let targets = routing::resolve(
        &config,
        &request.model,
        routing::required_capabilities(&request),
    )?;
    let context = RequestContext::new(state.request_id());
    let executor = HttpExecutor::from_state(state);
    let timeouts = executor.timeouts_for(&request.model);

    if request.stream {
        return stream(&executor, &targets, &request, &context, timeouts).await;
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
    let mut response = Json(to_anthropic(&routed.response)).into_response();
    crate::routes::add_routed_headers(
        &mut response,
        &routed.provider,
        &routed.model,
        routed.attempts,
    );
    Ok(response)
}

fn parse_request(body: &Value) -> Result<Request, GatewayError> {
    let model = body
        .get("model")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("`model` est requis"))?
        .to_owned();
    let max_tokens =
        body.get("max_tokens")
            .and_then(Value::as_u64)
            .ok_or_else(|| invalid("`max_tokens` est requis par l'API Anthropic"))? as u32;

    let mut messages = Vec::new();
    if let Some(system) = body.get("system") {
        let text = match system {
            Value::String(text) => text.clone(),
            Value::Array(blocks) => blocks
                .iter()
                .filter_map(|block| block.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n"),
            _ => return Err(invalid("`system` invalide")),
        };
        if !text.is_empty() {
            messages.push(Message {
                role: Role::System,
                content: vec![ContentPart::Text { text }],
                name: None,
                extra: Map::new(),
            });
        }
    }

    let raw_messages = body
        .get("messages")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("`messages` est requis"))?;
    for message in raw_messages {
        let role = match message.get("role").and_then(Value::as_str) {
            Some("user") => Role::User,
            Some("assistant") => Role::Assistant,
            other => return Err(invalid(format!("rôle Anthropic inconnu : {other:?}"))),
        };
        messages.push(Message {
            role,
            content: parse_content(message.get("content"))?,
            name: message
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_owned),
            extra: Map::new(),
        });
    }

    let mut tools = Vec::new();
    if let Some(values) = body.get("tools").and_then(Value::as_array) {
        for tool in values {
            tools.push(Tool {
                name: tool
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid("outil Anthropic sans `name`"))?
                    .to_owned(),
                description: tool
                    .get("description")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                parameters: tool
                    .get("input_schema")
                    .cloned()
                    .unwrap_or(Value::Object(Map::new())),
                extra: Map::new(),
            });
        }
    }

    Ok(Request {
        model,
        messages,
        tools,
        tool_choice: parse_tool_choice(body.get("tool_choice"))?,
        max_tokens: Some(max_tokens),
        temperature: body
            .get("temperature")
            .and_then(Value::as_f64)
            .map(|value| value as f32),
        top_p: body
            .get("top_p")
            .and_then(Value::as_f64)
            .map(|value| value as f32),
        stop: body
            .get("stop_sequences")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
        stream: body.get("stream").and_then(Value::as_bool).unwrap_or(false),
        response_format: None,
        extra: Map::new(),
    })
}

fn parse_content(value: Option<&Value>) -> Result<Vec<ContentPart>, GatewayError> {
    match value {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::String(text)) => Ok(vec![ContentPart::Text { text: text.clone() }]),
        Some(Value::Array(blocks)) => {
            let mut content = Vec::with_capacity(blocks.len());
            for block in blocks {
                match block.get("type").and_then(Value::as_str) {
                    Some("text") => content.push(ContentPart::Text {
                        text: text_of(block.get("text")),
                    }),
                    Some("image") => {
                        let source = block.get("source").cloned().unwrap_or(Value::Null);
                        content.push(ContentPart::Image {
                            source: parse_image(&source)?,
                            detail: buntu_schema::ImageDetail::Auto,
                        });
                    }
                    Some("tool_use") => {
                        content.push(ContentPart::ToolCall(buntu_schema::ToolCall {
                            id: text_of(block.get("id")),
                            name: text_of(block.get("name")),
                            arguments: block.get("input").cloned().unwrap_or(Value::Null),
                        }))
                    }
                    Some("tool_result") => {
                        let result_content = match block.get("content") {
                            Some(Value::String(text)) => text.clone(),
                            Some(Value::Array(parts)) => parts
                                .iter()
                                .filter_map(|part| part.get("text").and_then(Value::as_str))
                                .collect::<Vec<_>>()
                                .join("\n"),
                            _ => String::new(),
                        };
                        content.push(ContentPart::ToolResult(buntu_schema::ToolResult {
                            tool_call_id: text_of(block.get("tool_use_id")),
                            content: result_content,
                            is_error: block
                                .get("is_error")
                                .and_then(Value::as_bool)
                                .unwrap_or(false),
                        }));
                    }
                    other => {
                        return Err(invalid(format!(
                            "bloc de contenu Anthropic non pris en charge : {other:?}"
                        )));
                    }
                }
            }
            Ok(content)
        }
        Some(other) => Err(invalid(format!("contenu Anthropic invalide : {other}"))),
    }
}

fn parse_image(source: &Value) -> Result<ImageSource, GatewayError> {
    match source.get("type").and_then(Value::as_str) {
        Some("base64") => Ok(ImageSource::Base64 {
            media_type: text_of(source.get("media_type")),
            data: text_of(source.get("data")),
        }),
        Some("url") => Ok(ImageSource::Url {
            url: text_of(source.get("url")),
        }),
        other => Err(invalid(format!(
            "source d'image Anthropic inconnue : {other:?}"
        ))),
    }
}

fn parse_tool_choice(value: Option<&Value>) -> Result<ToolChoice, GatewayError> {
    let Some(value) = value else {
        return Ok(ToolChoice::Auto);
    };
    match value.get("type").and_then(Value::as_str) {
        Some("auto") => Ok(ToolChoice::Auto),
        Some("any") => Ok(ToolChoice::Required),
        Some("none") => Ok(ToolChoice::None),
        Some("tool") => value
            .get("name")
            .and_then(Value::as_str)
            .map(|name| ToolChoice::Named(name.to_owned()))
            .ok_or_else(|| invalid("tool_choice `tool` sans `name`")),
        other => Err(invalid(format!(
            "tool_choice Anthropic inconnu : {other:?}"
        ))),
    }
}

fn to_anthropic(response: &buntu_schema::Response) -> Value {
    let mut content = Vec::new();
    if let Some(choice) = response.choices.first() {
        for part in &choice.message.content {
            match part {
                ContentPart::Text { text } => {
                    content.push(json!({ "type": "text", "text": text }));
                }
                ContentPart::ToolCall(call) => {
                    content.push(json!({
                        "type": "tool_use",
                        "id": call.id,
                        "name": call.name,
                        "input": call.arguments,
                    }));
                }
                ContentPart::Reasoning { text, signature } => {
                    content.push(json!({
                        "type": "thinking",
                        "thinking": text,
                        "signature": signature,
                    }));
                }
                ContentPart::Refusal { refusal } => {
                    content.push(json!({ "type": "text", "text": refusal }));
                }
                _ => {}
            }
        }
    }
    json!({
        "id": response.id,
        "type": "message",
        "role": "assistant",
        "model": response.model,
        "content": content,
        "stop_reason": stop_reason(&response.stop_reason),
        "stop_sequence": Value::Null,
        "usage": {
            "input_tokens": response.usage.input_tokens,
            "output_tokens": response.usage.output_tokens,
            "cache_read_input_tokens": response.usage.cached_input_tokens,
        },
    })
}

fn stop_reason(reason: &Option<StopReason>) -> Value {
    let value = match reason {
        Some(StopReason::EndTurn) => "end_turn",
        Some(StopReason::MaxTokens) => "max_tokens",
        Some(StopReason::StopSequence) => "stop_sequence",
        Some(StopReason::ToolCalls) => "tool_use",
        Some(StopReason::Pause) => "pause_turn",
        Some(StopReason::Refusal) | Some(StopReason::ContentFilter) => "refusal",
        Some(StopReason::Other(other)) => other.as_str(),
        None => return Value::Null,
    };
    Value::String(value.to_owned())
}

async fn stream(
    executor: &HttpExecutor,
    targets: &[buntu_core::routing::Target],
    request: &Request,
    context: &RequestContext,
    timeouts: Timeouts,
) -> Result<AxumResponse, GatewayError> {
    let mut opened = None;
    let mut last_error = None;
    for target in targets {
        match executor.open_stream(target, request, timeouts).await {
            Ok((provider, chunks)) => {
                opened = Some((provider, target.clone(), chunks));
                break;
            }
            Err(error) => last_error = Some(error),
        }
    }
    let Some((provider, target, mut chunks)) = opened else {
        return Err(last_error.unwrap_or_else(|| GatewayError::Internal {
            message: "aucune cible pour le flux".to_owned(),
        }));
    };

    let id = format!("msg_{}", context.id);
    let model = target.model.clone();
    let stream = async_stream::stream! {
        let mut stream_state = StreamState::new();
        // (index du bloc ouvert, est-ce un bloc outil ?)
        let mut block: Option<(u32, bool)> = None;
        let mut next_index = 0_u32;
        let mut tool_id = String::new();
        let mut tool_name = String::new();
        let mut stop: Option<StopReason> = None;
        let mut output_tokens = 0_u32;
        let mut start_emitted = false;

        while let Some(next) = chunks.next().await {
            let events = match next.and_then(|data| provider.parse_stream_chunk(&mut stream_state, data)) {
                Ok(events) => events,
                Err(error) => {
                    yield Ok::<Bytes, std::convert::Infallible>(sse("error", &json!({
                        "type": "error",
                        "error": { "type": "api_error", "message": error.to_string() },
                    })));
                    break;
                }
            };
            for event in events {
                match event {
                    buntu_schema::StreamEvent::Start { .. } => {
                        start_emitted = true;
                        yield Ok(sse("message_start", &json!({
                            "type": "message_start",
                            "message": {
                                "id": id,
                                "type": "message",
                                "role": "assistant",
                                "model": model,
                                "content": [],
                                "stop_reason": Value::Null,
                                "stop_sequence": Value::Null,
                                "usage": { "input_tokens": 0, "output_tokens": 0 },
                            },
                        })));
                    }
                    buntu_schema::StreamEvent::ContentDelta { part, .. } => {
                        if !start_emitted {
                            continue;
                        }
                        let text = match part {
                            ContentPart::Text { text } => text,
                            ContentPart::Reasoning { text, .. } => text,
                            _ => continue,
                        };
                        if let Some((index, true)) = block {
                            yield Ok(sse("content_block_stop", &json!({
                                "type": "content_block_stop",
                                "index": index,
                            })));
                            block = None;
                        }
                        let index = match block {
                            Some((index, false)) => index,
                            _ => {
                                let index = next_index;
                                next_index += 1;
                                block = Some((index, false));
                                yield Ok(sse("content_block_start", &json!({
                                    "type": "content_block_start",
                                    "index": index,
                                    "content_block": { "type": "text", "text": "" },
                                })));
                                index
                            }
                        };
                        yield Ok(sse("content_block_delta", &json!({
                            "type": "content_block_delta",
                            "index": index,
                            "delta": { "type": "text_delta", "text": text },
                        })));
                    }
                    buntu_schema::StreamEvent::ToolCallDelta {
                        id: call_id,
                        name,
                        arguments_delta,
                        ..
                    } => {
                        if let Some(value) = call_id {
                            tool_id = value;
                        }
                        if let Some(value) = name {
                            tool_name = value;
                        }
                        if let Some((index, false)) = block {
                            yield Ok(sse("content_block_stop", &json!({
                                "type": "content_block_stop",
                                "index": index,
                            })));
                            block = None;
                        }
                        let index = match block {
                            Some((index, true)) => index,
                            _ => {
                                let index = next_index;
                                next_index += 1;
                                block = Some((index, true));
                                yield Ok(sse("content_block_start", &json!({
                                    "type": "content_block_start",
                                    "index": index,
                                    "content_block": {
                                        "type": "tool_use",
                                        "id": tool_id,
                                        "name": tool_name,
                                        "input": {},
                                    },
                                })));
                                index
                            }
                        };
                        if let Some(partial) = arguments_delta
                            && !partial.is_empty()
                        {
                            yield Ok(sse("content_block_delta", &json!({
                                "type": "content_block_delta",
                                "index": index,
                                "delta": {
                                    "type": "input_json_delta",
                                    "partial_json": partial,
                                },
                            })));
                        }
                    }
                    buntu_schema::StreamEvent::Usage { usage } => {
                        output_tokens = usage.output_tokens;
                    }
                    buntu_schema::StreamEvent::Stop { reason } => {
                        stop = Some(reason);
                    }
                    buntu_schema::StreamEvent::Done => {}
                }
            }
        }

        if let Some((index, _)) = block {
            yield Ok(sse("content_block_stop", &json!({
                "type": "content_block_stop",
                "index": index,
            })));
        }
        yield Ok(sse("message_delta", &json!({
            "type": "message_delta",
            "delta": { "stop_reason": stop_reason(&stop), "stop_sequence": Value::Null },
            "usage": { "output_tokens": output_tokens },
        })));
        yield Ok(sse("message_stop", &json!({ "type": "message_stop" })));
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

fn sse(event: &str, data: &Value) -> Bytes {
    Bytes::from(format!("event: {event}\ndata: {data}\n\n"))
}

fn text_of(value: Option<&Value>) -> String {
    value.and_then(Value::as_str).unwrap_or_default().to_owned()
}

fn invalid(message: impl Into<String>) -> GatewayError {
    GatewayError::InvalidRequest {
        message: message.into(),
    }
}
