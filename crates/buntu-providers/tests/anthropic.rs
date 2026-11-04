//! Tests unitaires de la traduction Anthropic (en plus de la batterie).

use std::error::Error;

use buntu_providers::anthropic::AnthropicProvider;
use buntu_providers::provider::{Provider, StreamState};
use buntu_schema::{
    ContentPart, ImageDetail, ImageSource, Message, Request, Role, StopReason, ToolResult, Usage,
};
use serde_json::{Map, Value, json};

fn request(messages: Vec<Message>) -> Request {
    Request {
        model: "claude-3-5-sonnet".to_owned(),
        messages,
        tools: Vec::new(),
        tool_choice: Default::default(),
        max_tokens: None,
        temperature: None,
        top_p: None,
        stop: Vec::new(),
        stream: false,
        response_format: None,
        extra: Map::new(),
    }
}

fn message(role: Role, parts: Vec<ContentPart>) -> Message {
    Message {
        role,
        content: parts,
        name: None,
        extra: Map::new(),
    }
}

#[test]
fn system_is_extracted_and_max_tokens_has_a_default() -> Result<(), Box<dyn Error>> {
    let request = request(vec![
        message(
            Role::System,
            vec![ContentPart::Text {
                text: "Tu es bref.".to_owned(),
            }],
        ),
        message(
            Role::User,
            vec![ContentPart::Text {
                text: "Bonjour".to_owned(),
            }],
        ),
    ]);
    let built = AnthropicProvider::default().build_request(&request)?;
    assert_eq!(
        built
            .headers()
            .get("anthropic-version")
            .and_then(|v| v.to_str().ok()),
        Some("2023-06-01")
    );
    let body: Value = serde_json::from_slice(built.body())?;
    assert_eq!(body["system"], "Tu es bref.");
    assert_eq!(body["max_tokens"], 4096);
    assert_eq!(body["messages"][0]["role"], "user");
    Ok(())
}

#[test]
fn tool_result_becomes_a_user_turn() -> Result<(), Box<dyn Error>> {
    let request = request(vec![message(
        Role::Tool,
        vec![ContentPart::ToolResult(ToolResult {
            tool_call_id: "call-1".to_owned(),
            content: "22 °C".to_owned(),
            is_error: false,
        })],
    )]);
    let built = AnthropicProvider::default().build_request(&request)?;
    let body: Value = serde_json::from_slice(built.body())?;
    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(body["messages"][0]["content"][0]["type"], "tool_result");
    assert_eq!(body["messages"][0]["content"][0]["tool_use_id"], "call-1");
    Ok(())
}

#[test]
fn image_url_uses_the_url_source() -> Result<(), Box<dyn Error>> {
    let request = request(vec![message(
        Role::User,
        vec![ContentPart::Image {
            source: ImageSource::Url {
                url: "https://exemple.test/photo.png".to_owned(),
            },
            detail: ImageDetail::Auto,
        }],
    )]);
    let built = AnthropicProvider::default().build_request(&request)?;
    let body: Value = serde_json::from_slice(built.body())?;
    assert_eq!(
        body["messages"][0]["content"][0]["source"],
        json!({"type": "url", "url": "https://exemple.test/photo.png"})
    );
    Ok(())
}

#[test]
fn response_maps_thinking_signature_and_cache_usage() -> Result<(), Box<dyn Error>> {
    let payload = br#"{
        "id": "msg-1",
        "type": "message",
        "role": "assistant",
        "model": "claude-3-5-sonnet",
        "content": [
            {"type": "thinking", "thinking": "raisonnement", "signature": "sig-1"},
            {"type": "text", "text": "Bonjour"}
        ],
        "stop_reason": "end_turn",
        "usage": {"input_tokens": 5, "output_tokens": 2, "cache_read_input_tokens": 4}
    }"#;
    let response =
        AnthropicProvider::default().parse_response(bytes::Bytes::from_static(payload))?;
    assert_eq!(response.stop_reason, Some(StopReason::EndTurn));
    assert_eq!(
        response.usage,
        Usage {
            input_tokens: 5,
            output_tokens: 2,
            cached_input_tokens: 4,
            reasoning_tokens: 0,
        }
    );
    match &response.choices[0].message.content[0] {
        ContentPart::Reasoning { text, signature } => {
            assert_eq!(text, "raisonnement");
            assert_eq!(signature.as_deref(), Some("sig-1"));
        }
        other => panic!("bloc inattendu : {other:?}"),
    }
    Ok(())
}

#[test]
fn stream_emits_start_deltas_stop_and_done() -> Result<(), Box<dyn Error>> {
    let sse = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m1\",\"model\":\"claude\",\"usage\":{\"input_tokens\":1}}}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Bon\"}}\n\n",
        "event: message_delta\n",
        "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":2}}\n\n",
        "event: message_stop\n",
        "data: {\"type\":\"message_stop\"}\n\n",
    );
    let mut state = StreamState::new();
    let mut events = Vec::new();
    for chunk in sse.as_bytes().chunks(11) {
        events.extend(
            AnthropicProvider::default()
                .parse_stream_chunk(&mut state, bytes::Bytes::copy_from_slice(chunk))?,
        );
    }
    assert!(state.is_finished());
    assert!(matches!(&events[0], buntu_schema::StreamEvent::Start { id, .. } if id == "m1"));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, buntu_schema::StreamEvent::Done))
    );
    assert!(events.iter().any(|event| matches!(
        event,
        buntu_schema::StreamEvent::Stop {
            reason: StopReason::EndTurn
        }
    )));
    Ok(())
}
