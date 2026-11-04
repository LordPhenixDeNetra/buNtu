//! Traduction du flux SSE Anthropic (événements nommés) vers les événements canoniques.
//!
//! Référence : <https://docs.anthropic.com/en/api/messages-streaming> (consultée le
//! 2026-11-04). Le type d'événement est porté par le champ `type` de chaque `data:`, ce qui
//! évite de mémoriser la ligne `event:` à travers les morceaux.

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
            message: format!("ligne SSE Anthropic non UTF-8 : {error}"),
        })?;
        let Some(data) = text.strip_prefix("data: ") else {
            continue;
        };
        let value: Value = serde_json::from_str(data).map_err(|error| GatewayError::Internal {
            message: format!("événement Anthropic illisible : {error}"),
        })?;

        match value.get("type").and_then(Value::as_str) {
            Some("message_start") => {
                let message = value.get("message").cloned().unwrap_or(Value::Null);
                if !started {
                    events.push(StreamEvent::Start {
                        id: message
                            .get("id")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        model: message
                            .get("model")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                    });
                    started = true;
                }
                events.push(StreamEvent::Usage {
                    usage: parse_usage(message.get("usage")),
                });
            }
            Some("content_block_start") => {
                let index = value.get("index").and_then(Value::as_u64).unwrap_or(0) as u32;
                let block = value.get("content_block").cloned().unwrap_or(Value::Null);
                if block.get("type").and_then(Value::as_str) == Some("tool_use") {
                    events.push(StreamEvent::ToolCallDelta {
                        index,
                        id: block.get("id").and_then(Value::as_str).map(str::to_owned),
                        name: block.get("name").and_then(Value::as_str).map(str::to_owned),
                        arguments_delta: None,
                    });
                }
            }
            Some("content_block_delta") => {
                let index = value.get("index").and_then(Value::as_u64).unwrap_or(0) as u32;
                let delta = value.get("delta").cloned().unwrap_or(Value::Null);
                match delta.get("type").and_then(Value::as_str) {
                    Some("text_delta") => events.push(StreamEvent::ContentDelta {
                        index,
                        part: ContentPart::Text {
                            text: delta
                                .get("text")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_owned(),
                        },
                    }),
                    Some("input_json_delta") => events.push(StreamEvent::ToolCallDelta {
                        index,
                        id: None,
                        name: None,
                        arguments_delta: delta
                            .get("partial_json")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                    }),
                    Some("thinking_delta") => events.push(StreamEvent::ContentDelta {
                        index,
                        part: ContentPart::Reasoning {
                            text: delta
                                .get("thinking")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_owned(),
                            signature: None,
                        },
                    }),
                    _ => {}
                }
            }
            Some("message_delta") => {
                let delta = value.get("delta").cloned().unwrap_or(Value::Null);
                if let Some(reason) = delta.get("stop_reason").and_then(Value::as_str) {
                    events.push(StreamEvent::Stop {
                        reason: stop_reason_of(reason),
                    });
                }
                if let Some(usage) = value.get("usage") {
                    events.push(StreamEvent::Usage {
                        usage: parse_usage(Some(usage)),
                    });
                }
            }
            Some("message_stop") => {
                state.mark_finished();
                events.push(StreamEvent::Done);
            }
            Some("error") => {
                let message = value
                    .get("error")
                    .and_then(|error| error.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or("erreur pendant le flux Anthropic")
                    .to_owned();
                return Err(GatewayError::ProviderUnavailable { message });
            }
            // `ping`, `content_block_stop` : rien à traduire.
            _ => {}
        }
    }

    state.record_events(events.len());
    Ok(events)
}
