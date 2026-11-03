//! Tests unitaires de la traduction OpenAI (en plus de la batterie de conformité).

use std::error::Error;

use buntu_providers::openai::OpenAiProvider;
use buntu_providers::provider::{Provider, StreamState};
use buntu_schema::{
    ContentPart, GatewayError, ImageDetail, ImageSource, Message, Request, Role, StopReason,
    StreamEvent, Tool, ToolChoice, ToolResult, Usage,
};
use serde_json::{Map, Value, json};

fn provider() -> OpenAiProvider {
    OpenAiProvider::default()
}

fn request(content: Vec<ContentPart>) -> Request {
    Request {
        model: "gpt-4o-mini".to_owned(),
        messages: vec![Message {
            role: Role::User,
            content,
            name: None,
            extra: Map::new(),
        }],
        tools: Vec::new(),
        tool_choice: ToolChoice::Auto,
        max_tokens: None,
        temperature: None,
        top_p: None,
        stop: Vec::new(),
        stream: false,
        response_format: None,
        extra: Map::new(),
    }
}

fn built_body(request: &Request) -> Result<Value, Box<dyn Error>> {
    let built = provider().build_request(request)?;
    Ok(serde_json::from_slice(built.body())?)
}

#[test]
fn request_serializes_messages_tools_and_stream_options() -> Result<(), Box<dyn Error>> {
    let mut request = request(vec![ContentPart::Text {
        text: "Bonjour".to_owned(),
    }]);
    request.stream = true;
    request.tools.push(Tool {
        name: "meteo".to_owned(),
        description: Some("Météo d'une ville".to_owned()),
        parameters: json!({"type": "object"}),
        extra: Map::new(),
    });
    request.tool_choice = ToolChoice::Named("meteo".to_owned());

    let body = built_body(&request)?;
    assert_eq!(body["model"], "gpt-4o-mini");
    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(body["messages"][0]["content"], "Bonjour");
    assert_eq!(body["stream"], true);
    assert_eq!(body["stream_options"]["include_usage"], true);
    assert_eq!(body["tools"][0]["function"]["name"], "meteo");
    assert_eq!(body["tool_choice"]["function"]["name"], "meteo");
    Ok(())
}

#[test]
fn base64_image_becomes_a_data_uri() -> Result<(), Box<dyn Error>> {
    let request = request(vec![ContentPart::Image {
        source: ImageSource::Base64 {
            media_type: "image/png".to_owned(),
            data: "AAAA".to_owned(),
        },
        detail: ImageDetail::Auto,
    }]);
    let body = built_body(&request)?;
    let url = body["messages"][0]["content"][0]["image_url"]["url"]
        .as_str()
        .unwrap_or_default();
    assert!(url.starts_with("data:image/png;base64,"), "url : {url}");
    Ok(())
}

#[test]
fn tool_result_becomes_a_tool_message() -> Result<(), Box<dyn Error>> {
    let mut request = request(Vec::new());
    request.messages.push(Message {
        role: Role::Tool,
        content: vec![ContentPart::ToolResult(ToolResult {
            tool_call_id: "call-1".to_owned(),
            content: "22 °C".to_owned(),
            is_error: false,
        })],
        name: None,
        extra: Map::new(),
    });

    let body = built_body(&request)?;
    assert_eq!(
        body["messages"][1],
        json!({"role": "tool", "tool_call_id": "call-1", "content": "22 °C"})
    );
    Ok(())
}

#[test]
fn response_parses_tool_calls_and_usage_details() -> Result<(), Box<dyn Error>> {
    let payload = br#"{
        "id": "chatcmpl-1",
        "model": "gpt-4o-mini",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call-1",
                    "type": "function",
                    "function": {"name": "meteo", "arguments": "{\"ville\":\"Dakar\"}"}
                }]
            },
            "finish_reason": "tool_calls"
        }],
        "usage": {
            "prompt_tokens": 12,
            "completion_tokens": 8,
            "prompt_tokens_details": {"cached_tokens": 9},
            "completion_tokens_details": {"reasoning_tokens": 7}
        }
    }"#;
    let response = provider().parse_response(bytes::Bytes::from_static(payload))?;

    assert_eq!(response.id, "chatcmpl-1");
    assert_eq!(response.stop_reason, Some(StopReason::ToolCalls));
    assert_eq!(
        response.usage,
        Usage {
            input_tokens: 12,
            output_tokens: 8,
            cached_input_tokens: 9,
            reasoning_tokens: 7,
        }
    );
    let message = &response.choices[0].message;
    match &message.content[0] {
        ContentPart::ToolCall(call) => {
            assert_eq!(call.id, "call-1");
            assert_eq!(call.name, "meteo");
            assert_eq!(call.arguments, json!({"ville": "Dakar"}));
        }
        other => panic!("contenu inattendu : {other:?}"),
    }
    Ok(())
}

#[test]
fn stream_chunks_can_be_split_anywhere() -> Result<(), Box<dyn Error>> {
    let sse = concat!(
        "data: {\"id\":\"chatcmpl-1\",\"model\":\"gpt-4o-mini\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Bon\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-1\",\"model\":\"gpt-4o-mini\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"jour\"},\"finish_reason\":null}]}\n\n",
        "data: [DONE]\n\n",
    );

    let mut state = StreamState::new();
    let mut events = Vec::new();
    for chunk in sse.as_bytes().chunks(7) {
        events.extend(
            provider().parse_stream_chunk(&mut state, bytes::Bytes::copy_from_slice(chunk))?,
        );
    }

    assert!(state.is_finished(), "le flux doit être terminé");
    assert_eq!(events.len(), 4, "événements : {events:?}");
    assert!(matches!(&events[0], StreamEvent::Start { id, .. } if id == "chatcmpl-1"));
    assert!(
        matches!(&events[1], StreamEvent::ContentDelta { part: ContentPart::Text { text }, .. } if text == "Bon")
    );
    assert!(
        matches!(&events[2], StreamEvent::ContentDelta { part: ContentPart::Text { text }, .. } if text == "jour")
    );
    assert!(matches!(&events[3], StreamEvent::Done));
    Ok(())
}

#[test]
fn errors_are_classified() -> Result<(), Box<dyn Error>> {
    let provider = provider();
    let mut headers = hyper::HeaderMap::new();
    headers.insert("retry-after", "2".parse()?);
    let rate_limited = provider.map_error(429, &headers, b"{\"error\":{\"message\":\"trop\"}}");
    match rate_limited {
        GatewayError::ProviderRateLimited { retry_after } => {
            assert_eq!(retry_after, Some(std::time::Duration::from_secs(2)));
        }
        other => panic!("erreur inattendue : {other:?}"),
    }

    let invalid = provider.map_error(
        400,
        &hyper::HeaderMap::new(),
        b"{\"error\":{\"message\":\"invalide\"}}",
    );
    match invalid {
        GatewayError::InvalidRequest { message } => assert!(message.contains("invalide")),
        other => panic!("erreur inattendue : {other:?}"),
    }
    Ok(())
}
