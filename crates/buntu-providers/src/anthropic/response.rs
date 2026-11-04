//! Traduction de la réponse Anthropic Messages vers le canonique.
//!
//! Référence : <https://docs.anthropic.com/en/api/messages> (consultée le 2026-11-04).

use buntu_schema::{
    Choice, ContentPart, GatewayError, Message, Response, Role, StopReason, ToolCall, Usage,
};
use serde_json::Value;

pub fn parse(body: &[u8]) -> Result<Response, GatewayError> {
    let value: Value = serde_json::from_slice(body).map_err(|error| GatewayError::Internal {
        message: format!("réponse Anthropic illisible : {error}"),
    })?;

    let blocks = value
        .get("content")
        .and_then(Value::as_array)
        .ok_or_else(|| GatewayError::Internal {
            message: "réponse Anthropic sans `content`".to_owned(),
        })?;

    let mut content = Vec::with_capacity(blocks.len());
    for block in blocks {
        match block.get("type").and_then(Value::as_str) {
            Some("text") => content.push(ContentPart::Text {
                text: text(block.get("text")),
            }),
            Some("tool_use") => content.push(ContentPart::ToolCall(ToolCall {
                id: text(block.get("id")),
                name: text(block.get("name")),
                arguments: block.get("input").cloned().unwrap_or(Value::Null),
            })),
            Some("thinking") => content.push(ContentPart::Reasoning {
                text: text(block.get("thinking")),
                signature: block
                    .get("signature")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            }),
            Some("redacted_thinking") => content.push(ContentPart::Reasoning {
                text: String::new(),
                signature: block.get("data").and_then(Value::as_str).map(str::to_owned),
            }),
            other => {
                return Err(GatewayError::Internal {
                    message: format!("bloc de contenu Anthropic inconnu : {other:?}"),
                });
            }
        }
    }

    Ok(Response {
        id: text(value.get("id")),
        model: text(value.get("model")),
        choices: vec![Choice {
            index: 0,
            message: Message {
                role: Role::Assistant,
                content,
                name: None,
                extra: serde_json::Map::new(),
            },
        }],
        usage: parse_usage(value.get("usage")),
        stop_reason: value
            .get("stop_reason")
            .and_then(Value::as_str)
            .map(stop_reason_of),
        extra: serde_json::Map::new(),
    })
}

fn text(value: Option<&Value>) -> String {
    value.and_then(Value::as_str).unwrap_or_default().to_owned()
}

pub fn stop_reason_of(reason: &str) -> StopReason {
    match reason {
        "end_turn" => StopReason::EndTurn,
        "max_tokens" => StopReason::MaxTokens,
        "stop_sequence" => StopReason::StopSequence,
        "tool_use" => StopReason::ToolCalls,
        "pause_turn" => StopReason::Pause,
        "refusal" => StopReason::Refusal,
        other => StopReason::Other(other.to_owned()),
    }
}

pub fn parse_usage(value: Option<&Value>) -> Usage {
    let Some(usage) = value else {
        return Usage::default();
    };
    Usage {
        input_tokens: number(usage.get("input_tokens")),
        output_tokens: number(usage.get("output_tokens")),
        cached_input_tokens: number(usage.get("cache_read_input_tokens")),
        reasoning_tokens: 0,
    }
}

fn number(value: Option<&Value>) -> u32 {
    value.and_then(Value::as_u64).unwrap_or(0) as u32
}
