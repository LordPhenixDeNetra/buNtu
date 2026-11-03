//! Traduction de la réponse OpenAI Chat Completions vers le canonique.
//!
//! Référence : <https://platform.openai.com/docs/api-reference/chat/object> (consultée le
//! 2026-11-03), champs `choices[].message.content`, `tool_calls`, `usage.prompt_tokens_details`.

use buntu_schema::{
    Choice, ContentPart, GatewayError, Message, Response, Role, StopReason, ToolCall, Usage,
};
use serde_json::Value;

pub fn parse(body: &[u8]) -> Result<Response, GatewayError> {
    let value: Value = serde_json::from_slice(body).map_err(|error| GatewayError::Internal {
        message: format!("réponse OpenAI illisible : {error}"),
    })?;

    let choices = value
        .get("choices")
        .and_then(Value::as_array)
        .ok_or_else(|| GatewayError::Internal {
            message: "réponse OpenAI sans `choices`".to_owned(),
        })?;

    let mut parsed = Vec::with_capacity(choices.len());
    let mut stop_reason = None;
    for (position, choice) in choices.iter().enumerate() {
        let message = choice
            .get("message")
            .ok_or_else(|| GatewayError::Internal {
                message: "choix OpenAI sans `message`".to_owned(),
            })?;
        parsed.push(Choice {
            index: choice
                .get("index")
                .and_then(Value::as_u64)
                .unwrap_or(position as u64) as u32,
            message: parse_message(message)?,
        });
        if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
            stop_reason = Some(stop_reason_of(reason));
        }
    }

    Ok(Response {
        id: text(value.get("id")),
        model: text(value.get("model")),
        choices: parsed,
        usage: parse_usage(value.get("usage")),
        stop_reason,
        extra: serde_json::Map::new(),
    })
}

fn text(value: Option<&Value>) -> String {
    value.and_then(Value::as_str).unwrap_or_default().to_owned()
}

fn parse_message(value: &Value) -> Result<Message, GatewayError> {
    let role = match value.get("role").and_then(Value::as_str) {
        Some("assistant") | None => Role::Assistant,
        Some("user") => Role::User,
        Some("system") => Role::System,
        Some("tool") => Role::Tool,
        Some(other) => {
            return Err(GatewayError::Internal {
                message: format!("rôle OpenAI inconnu : {other}"),
            });
        }
    };

    let mut content = Vec::new();
    match value.get("content") {
        Some(Value::String(value)) if !value.is_empty() => {
            content.push(ContentPart::Text {
                text: value.clone(),
            });
        }
        Some(Value::Array(parts)) => {
            for part in parts {
                match part.get("type").and_then(Value::as_str) {
                    Some("text") => content.push(ContentPart::Text {
                        text: text(part.get("text")),
                    }),
                    Some("refusal") => content.push(ContentPart::Refusal {
                        refusal: text(part.get("refusal")),
                    }),
                    other => {
                        return Err(GatewayError::Internal {
                            message: format!("type de contenu OpenAI inconnu : {other:?}"),
                        });
                    }
                }
            }
        }
        _ => {}
    }

    if let Some(refusal) = value.get("refusal").and_then(Value::as_str)
        && !refusal.is_empty()
    {
        content.push(ContentPart::Refusal {
            refusal: refusal.to_owned(),
        });
    }

    if let Some(calls) = value.get("tool_calls").and_then(Value::as_array) {
        for call in calls {
            let function = call.get("function").cloned().unwrap_or(Value::Null);
            let arguments = function
                .get("arguments")
                .and_then(Value::as_str)
                .map(|raw| {
                    serde_json::from_str::<Value>(raw)
                        .unwrap_or_else(|_| Value::String(raw.to_owned()))
                })
                .unwrap_or(Value::Null);
            content.push(ContentPart::ToolCall(ToolCall {
                id: text(call.get("id")),
                name: text(function.get("name")),
                arguments,
            }));
        }
    }

    Ok(Message {
        role,
        content,
        name: value.get("name").and_then(Value::as_str).map(str::to_owned),
        extra: serde_json::Map::new(),
    })
}

pub fn stop_reason_of(reason: &str) -> StopReason {
    match reason {
        "stop" => StopReason::EndTurn,
        "length" => StopReason::MaxTokens,
        "tool_calls" | "function_call" => StopReason::ToolCalls,
        "content_filter" => StopReason::ContentFilter,
        other => StopReason::Other(other.to_owned()),
    }
}

pub fn parse_usage(value: Option<&Value>) -> Usage {
    let Some(usage) = value else {
        return Usage::default();
    };
    Usage {
        input_tokens: number(usage.get("prompt_tokens")),
        output_tokens: number(usage.get("completion_tokens")),
        cached_input_tokens: number(
            usage
                .get("prompt_tokens_details")
                .and_then(|details| details.get("cached_tokens")),
        ),
        reasoning_tokens: number(
            usage
                .get("completion_tokens_details")
                .and_then(|details| details.get("reasoning_tokens")),
        ),
    }
}

fn number(value: Option<&Value>) -> u32 {
    value.and_then(Value::as_u64).unwrap_or(0) as u32
}
