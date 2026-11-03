//! Traduction du flux SSE OpenAI (`chat.completion.chunk`) vers les événements canoniques.
//!
//! Référence : <https://platform.openai.com/docs/api-reference/chat/streaming> (consultée le
//! 2026-11-03) : chaque `data:` porte un chunk, le flux se termine par `data: [DONE]`, et un
//! frame peut contenir un objet `error` si l'erreur survient après le début du flux.

use buntu_schema::{ContentPart, GatewayError, StreamEvent};
use bytes::Bytes;
use serde_json::Value;

use crate::provider::StreamState;

use super::response::{parse_usage, stop_reason_of};

pub fn parse(state: &mut StreamState, chunk: Bytes) -> Result<Vec<StreamEvent>, GatewayError> {
    state.push(&chunk);
    let mut events = Vec::new();
    let mut started = state.events_seen() > 0;

    for line in state.take_lines() {
        let text = std::str::from_utf8(&line).map_err(|error| GatewayError::Internal {
            message: format!("ligne SSE OpenAI non UTF-8 : {error}"),
        })?;
        let Some(data) = text.strip_prefix("data: ") else {
            continue;
        };

        if data.trim() == "[DONE]" {
            state.mark_finished();
            events.push(StreamEvent::Done);
            continue;
        }

        let value: Value = serde_json::from_str(data).map_err(|error| GatewayError::Internal {
            message: format!("chunk OpenAI illisible : {error}"),
        })?;

        if let Some(error) = value.get("error") {
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("erreur pendant le flux")
                .to_owned();
            return Err(GatewayError::ProviderUnavailable { message });
        }

        if !started {
            events.push(StreamEvent::Start {
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
            });
            started = true;
        }

        if let Some(choices) = value.get("choices").and_then(Value::as_array) {
            for choice in choices {
                let index = choice.get("index").and_then(Value::as_u64).unwrap_or(0) as u32;
                let delta = choice.get("delta").cloned().unwrap_or(Value::Null);

                if let Some(content) = delta.get("content").and_then(Value::as_str)
                    && !content.is_empty()
                {
                    events.push(StreamEvent::ContentDelta {
                        index,
                        part: ContentPart::Text {
                            text: content.to_owned(),
                        },
                    });
                }

                if let Some(calls) = delta.get("tool_calls").and_then(Value::as_array) {
                    for call in calls {
                        events.push(StreamEvent::ToolCallDelta {
                            index: call.get("index").and_then(Value::as_u64).unwrap_or(0) as u32,
                            id: call.get("id").and_then(Value::as_str).map(str::to_owned),
                            name: call
                                .get("function")
                                .and_then(|function| function.get("name"))
                                .and_then(Value::as_str)
                                .map(str::to_owned),
                            arguments_delta: call
                                .get("function")
                                .and_then(|function| function.get("arguments"))
                                .and_then(Value::as_str)
                                .map(str::to_owned),
                        });
                    }
                }

                if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
                    events.push(StreamEvent::Stop {
                        reason: stop_reason_of(reason),
                    });
                }
            }
        }

        if let Some(usage) = value.get("usage").filter(|usage| !usage.is_null()) {
            events.push(StreamEvent::Usage {
                usage: parse_usage(Some(usage)),
            });
        }
    }

    state.record_events(events.len());
    Ok(events)
}
