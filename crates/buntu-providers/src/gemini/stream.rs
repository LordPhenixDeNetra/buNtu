//! Traduction du flux SSE Gemini (`streamGenerateContent?alt=sse`) vers le canonique.
//!
//! Référence : <https://ai.google.dev/gemini-api/docs/streaming> (consultée le 2026-11-04).

use buntu_schema::{ContentPart, GatewayError, StreamEvent};
use bytes::Bytes;
use serde_json::Value;

use crate::provider::StreamState;

use super::response::{parse_parts, parse_usage, stop_reason_of};

pub fn parse(state: &mut StreamState, chunk: Bytes) -> Result<Vec<StreamEvent>, GatewayError> {
    state.push(&chunk);
    let mut events = Vec::new();
    let mut started = state.events_seen() > 0;

    for line in state.take_lines() {
        let text = std::str::from_utf8(&line).map_err(|error| GatewayError::Internal {
            message: format!("ligne SSE Gemini non UTF-8 : {error}"),
        })?;
        let Some(data) = text.strip_prefix("data: ") else {
            continue;
        };
        let value: Value = serde_json::from_str(data).map_err(|error| GatewayError::Internal {
            message: format!("chunk Gemini illisible : {error}"),
        })?;

        if let Some(error) = value.get("error") {
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("erreur pendant le flux Gemini")
                .to_owned();
            return Err(GatewayError::ProviderUnavailable { message });
        }

        if !started {
            events.push(StreamEvent::Start {
                id: value
                    .get("responseId")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                model: value
                    .get("modelVersion")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            });
            started = true;
        }

        let candidate = value
            .get("candidates")
            .and_then(Value::as_array)
            .and_then(|candidates| candidates.first())
            .cloned()
            .unwrap_or(Value::Null);

        let parts = parse_parts(
            candidate
                .get("content")
                .and_then(|content| content.get("parts")),
        )?;
        for (index, part) in parts.into_iter().enumerate() {
            match part {
                ContentPart::Text { text } => {
                    if !text.is_empty() {
                        events.push(StreamEvent::ContentDelta {
                            index: index as u32,
                            part: ContentPart::Text { text },
                        });
                    }
                }
                ContentPart::ToolCall(call) => {
                    let arguments = serde_json::to_string(&call.arguments).unwrap_or_default();
                    events.push(StreamEvent::ToolCallDelta {
                        index: index as u32,
                        id: Some(call.id),
                        name: Some(call.name),
                        arguments_delta: Some(arguments),
                    });
                }
                _ => {}
            }
        }

        if let Some(usage) = value.get("usageMetadata") {
            events.push(StreamEvent::Usage {
                usage: parse_usage(Some(usage)),
            });
        }

        if let Some(reason) = candidate.get("finishReason").and_then(Value::as_str) {
            events.push(StreamEvent::Stop {
                reason: stop_reason_of(reason),
            });
            state.mark_finished();
            events.push(StreamEvent::Done);
        }
    }

    state.record_events(events.len());
    Ok(events)
}
