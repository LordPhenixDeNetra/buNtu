//! Traduction de la réponse Gemini vers le canonique.
//!
//! Référence : <https://ai.google.dev/api/generate-content> (consultée le 2026-11-04).

use buntu_schema::{
    Choice, ContentPart, GatewayError, ImageSource, Message, Response, Role, StopReason, ToolCall,
    Usage,
};
use serde_json::Value;

pub fn parse(body: &[u8]) -> Result<Response, GatewayError> {
    let value: Value = serde_json::from_slice(body).map_err(|error| GatewayError::Internal {
        message: format!("réponse Gemini illisible : {error}"),
    })?;

    let candidate = value
        .get("candidates")
        .and_then(Value::as_array)
        .and_then(|candidates| candidates.first())
        .cloned()
        .unwrap_or(Value::Null);

    let content = parse_parts(
        candidate
            .get("content")
            .and_then(|content| content.get("parts")),
    )?;
    let mut stop_reason = candidate
        .get("finishReason")
        .and_then(Value::as_str)
        .map(stop_reason_of);
    if content
        .iter()
        .any(|part| matches!(part, ContentPart::ToolCall(_)))
    {
        stop_reason = Some(StopReason::ToolCalls);
    }

    Ok(Response {
        id: text(value.get("responseId")),
        model: text(value.get("modelVersion")),
        choices: vec![Choice {
            index: 0,
            message: Message {
                role: Role::Assistant,
                content,
                name: None,
                extra: serde_json::Map::new(),
            },
        }],
        usage: parse_usage(value.get("usageMetadata")),
        stop_reason,
        extra: serde_json::Map::new(),
    })
}

fn text(value: Option<&Value>) -> String {
    value.and_then(Value::as_str).unwrap_or_default().to_owned()
}

pub fn parse_parts(value: Option<&Value>) -> Result<Vec<ContentPart>, GatewayError> {
    let mut content = Vec::new();
    let Some(parts) = value.and_then(Value::as_array) else {
        return Ok(content);
    };
    for (index, part) in parts.iter().enumerate() {
        if let Some(text) = part.get("text").and_then(Value::as_str) {
            content.push(ContentPart::Text {
                text: text.to_owned(),
            });
        } else if let Some(call) = part.get("functionCall") {
            content.push(ContentPart::ToolCall(ToolCall {
                // Gemini n'utilise pas d'identifiant d'appel : on en génère un stable pour la
                // corrélation canonique (documenté dans docs/providers/gemini.md).
                id: format!("call-{}", index + 1),
                name: text(call.get("name")),
                arguments: call.get("args").cloned().unwrap_or(Value::Null),
            }));
        } else if let Some(inline) = part.get("inlineData") {
            content.push(ContentPart::Image {
                source: ImageSource::Base64 {
                    media_type: text(inline.get("mimeType")),
                    data: text(inline.get("data")),
                },
                detail: buntu_schema::ImageDetail::Auto,
            });
        } else {
            return Err(GatewayError::Internal {
                message: "partie de réponse Gemini inconnue".to_owned(),
            });
        }
    }
    Ok(content)
}

pub fn stop_reason_of(reason: &str) -> StopReason {
    match reason {
        "STOP" => StopReason::EndTurn,
        "MAX_TOKENS" => StopReason::MaxTokens,
        "SAFETY" | "RECITATION" | "BLOCKLIST" | "PROHIBITED_CONTENT" => StopReason::ContentFilter,
        other => StopReason::Other(other.to_owned()),
    }
}

pub fn parse_usage(value: Option<&Value>) -> Usage {
    let Some(usage) = value else {
        return Usage::default();
    };
    Usage {
        input_tokens: number(usage.get("promptTokenCount")),
        output_tokens: number(usage.get("candidatesTokenCount")),
        cached_input_tokens: number(usage.get("cachedContentTokenCount")),
        reasoning_tokens: number(usage.get("thoughtsTokenCount")),
    }
}

fn number(value: Option<&Value>) -> u32 {
    value.and_then(Value::as_u64).unwrap_or(0) as u32
}
