//! Sérialisation canonique vers le format OpenAI (réponses et flux SSE).
//!
//! Référence : <https://platform.openai.com/docs/api-reference/chat/object> et
//! <https://platform.openai.com/docs/api-reference/chat/streaming> (consultées le 2026-11-04).

use buntu_schema::{ContentPart, Response, StopReason, StreamEvent};
use serde_json::{Value, json};

/// Corps d'une réponse chat complète.
pub fn to_openai(response: &Response) -> Value {
    let choices: Vec<Value> = response
        .choices
        .iter()
        .map(|choice| {
            let content: Option<String> = {
                let mut text = String::new();
                let mut has_text = false;
                for part in &choice.message.content {
                    if let ContentPart::Text { text: value } = part {
                        has_text = true;
                        text.push_str(value);
                    }
                }
                if has_text { Some(text) } else { None }
            };
            let tool_calls: Vec<Value> = choice
                .message
                .content
                .iter()
                .filter_map(|part| match part {
                    ContentPart::ToolCall(call) => Some(json!({
                        "id": call.id,
                        "type": "function",
                        "function": {
                            "name": call.name,
                            "arguments": serde_json::to_string(&call.arguments).unwrap_or_else(|_| "{}".to_owned()),
                        },
                    })),
                    _ => None,
                })
                .collect();
            json!({
                "index": choice.index,
                "message": {
                    "role": "assistant",
                    "content": content,
                    "tool_calls": if tool_calls.is_empty() { Value::Null } else { Value::Array(tool_calls) },
                },
                "finish_reason": finish_reason(&response.stop_reason),
            })
        })
        .collect();

    json!({
        "id": response.id,
        "object": "chat.completion",
        "created": created(),
        "model": response.model,
        "choices": choices,
        "usage": {
            "prompt_tokens": response.usage.input_tokens,
            "completion_tokens": response.usage.output_tokens,
            "total_tokens": response.usage.input_tokens + response.usage.output_tokens,
            "prompt_tokens_details": {
                "cached_tokens": response.usage.cached_input_tokens,
            },
            "completion_tokens_details": {
                "reasoning_tokens": response.usage.reasoning_tokens,
            },
        },
    })
}

/// Un chunk SSE OpenAI pour un événement canonique.
pub fn chunk(event: &StreamEvent, id: &str, model: &str) -> Option<Value> {
    match event {
        StreamEvent::Start { .. } => Some(json!({
            "id": id,
            "object": "chat.completion.chunk",
            "created": created(),
            "model": model,
            "choices": [{ "index": 0, "delta": { "role": "assistant", "content": "" }, "finish_reason": Value::Null }],
        })),
        StreamEvent::ContentDelta { index, part } => {
            let ContentPart::Text { text } = part else {
                return None;
            };
            Some(json!({
                "id": id,
                "object": "chat.completion.chunk",
                "created": created(),
                "model": model,
                "choices": [{ "index": index, "delta": { "content": text }, "finish_reason": Value::Null }],
            }))
        }
        StreamEvent::ToolCallDelta {
            index,
            id: call_id,
            name,
            arguments_delta,
        } => Some(json!({
            "id": id,
            "object": "chat.completion.chunk",
            "created": created(),
            "model": model,
            "choices": [{
                "index": 0,
                "delta": {
                    "tool_calls": [{
                        "index": index,
                        "id": call_id,
                        "type": "function",
                        "function": { "name": name, "arguments": arguments_delta },
                    }],
                },
                "finish_reason": Value::Null,
            }],
        })),
        StreamEvent::Stop { reason } => Some(json!({
            "id": id,
            "object": "chat.completion.chunk",
            "created": created(),
            "model": model,
            "choices": [{ "index": 0, "delta": {}, "finish_reason": finish_reason(&Some(reason.clone())) }],
        })),
        StreamEvent::Usage { usage } => Some(json!({
            "id": id,
            "object": "chat.completion.chunk",
            "created": created(),
            "model": model,
            "choices": [],
            "usage": {
                "prompt_tokens": usage.input_tokens,
                "completion_tokens": usage.output_tokens,
                "total_tokens": usage.input_tokens + usage.output_tokens,
            },
        })),
        StreamEvent::Done => None,
    }
}

/// Corps d'une réponse d'embeddings au format OpenAI.
pub fn embeddings_to_openai(response: &buntu_schema::EmbeddingsResponse) -> Value {
    json!({
        "object": "list",
        "data": response.data.iter().map(|embedding| json!({
            "object": "embedding",
            "index": embedding.index,
            "embedding": embedding.embedding,
        })).collect::<Vec<Value>>(),
        "model": response.model,
        "usage": {
            "prompt_tokens": response.usage.prompt_tokens,
            "total_tokens": response.usage.total_tokens,
        },
    })
}

fn finish_reason(reason: &Option<StopReason>) -> Value {
    let value = match reason {
        Some(StopReason::EndTurn) | Some(StopReason::Pause) => "stop",
        Some(StopReason::MaxTokens) => "length",
        Some(StopReason::ToolCalls) => "tool_calls",
        Some(StopReason::ContentFilter) | Some(StopReason::Refusal) => "content_filter",
        Some(StopReason::StopSequence) => "stop",
        Some(StopReason::Other(other)) => other.as_str(),
        None => return Value::Null,
    };
    Value::String(value.to_owned())
}

fn created() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
