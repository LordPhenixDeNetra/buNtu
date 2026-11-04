//! Tests unitaires de la traduction Gemini (en plus de la batterie).

use std::error::Error;

use buntu_providers::gemini::GeminiProvider;
use buntu_providers::provider::{Provider, StreamState};
use buntu_schema::{ContentPart, Message, Request, ResponseFormat, Role, StopReason, ToolChoice};
use serde_json::{Map, Value, json};

fn request(messages: Vec<Message>) -> Request {
    Request {
        model: "gemini-2.5-flash".to_owned(),
        messages,
        tools: Vec::new(),
        tool_choice: ToolChoice::Auto,
        max_tokens: Some(256),
        temperature: Some(0.2),
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
fn endpoint_contains_model_and_stream_suffix() -> Result<(), Box<dyn Error>> {
    let mut request = request(vec![message(
        Role::User,
        vec![ContentPart::Text {
            text: "Bonjour".to_owned(),
        }],
    )]);
    request.stream = true;
    let built = GeminiProvider::default().build_request(&request)?;
    let uri = built.uri().to_string();
    assert!(uri.starts_with("/v1beta/models/"), "uri : {uri}");
    assert!(
        uri.contains("gemini-2.5-flash:streamGenerateContent"),
        "uri : {uri}"
    );
    assert!(uri.contains("alt=sse"), "uri : {uri}");
    Ok(())
}

#[test]
fn system_message_becomes_system_instruction() -> Result<(), Box<dyn Error>> {
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
    let built = GeminiProvider::default().build_request(&request)?;
    let body: Value = serde_json::from_slice(built.body())?;
    assert_eq!(body["systemInstruction"]["parts"][0]["text"], "Tu es bref.");
    assert_eq!(body["contents"][0]["role"], "user");
    assert_eq!(body["generationConfig"]["maxOutputTokens"], 256);
    Ok(())
}

#[test]
fn tool_choice_named_restricts_allowed_functions() -> Result<(), Box<dyn Error>> {
    let mut request = request(vec![message(
        Role::User,
        vec![ContentPart::Text {
            text: "Météo ?".to_owned(),
        }],
    )]);
    request.tools.push(buntu_schema::Tool {
        name: "meteo".to_owned(),
        description: None,
        parameters: json!({"type": "object"}),
        extra: Map::new(),
    });
    request.tool_choice = ToolChoice::Named("meteo".to_owned());
    let built = GeminiProvider::default().build_request(&request)?;
    let body: Value = serde_json::from_slice(built.body())?;
    assert_eq!(body["toolConfig"]["functionCallingConfig"]["mode"], "ANY");
    assert_eq!(
        body["toolConfig"]["functionCallingConfig"]["allowedFunctionNames"][0],
        "meteo"
    );
    Ok(())
}

#[test]
fn json_schema_becomes_response_schema() -> Result<(), Box<dyn Error>> {
    let mut request = request(vec![message(
        Role::User,
        vec![ContentPart::Text {
            text: "Renvoie du JSON".to_owned(),
        }],
    )]);
    request.response_format = Some(ResponseFormat::JsonSchema {
        name: "sortie".to_owned(),
        schema: json!({"type": "object"}),
    });
    let built = GeminiProvider::default().build_request(&request)?;
    let body: Value = serde_json::from_slice(built.body())?;
    assert_eq!(
        body["generationConfig"]["responseMimeType"],
        "application/json"
    );
    assert_eq!(
        body["generationConfig"]["responseSchema"],
        json!({"type": "object"})
    );
    Ok(())
}

#[test]
fn response_generates_tool_ids_and_maps_safety() -> Result<(), Box<dyn Error>> {
    let payload = br#"{
        "candidates": [{
            "content": {"role": "model", "parts": [{"functionCall": {"name": "meteo", "args": {"ville": "Dakar"}}}]},
            "finishReason": "SAFETY",
            "index": 0
        }],
        "usageMetadata": {"promptTokenCount": 4, "candidatesTokenCount": 6},
        "modelVersion": "gemini-2.5-flash",
        "responseId": "resp-1"
    }"#;
    let response = GeminiProvider::default().parse_response(bytes::Bytes::from_static(payload))?;
    assert_eq!(response.id, "resp-1");
    // La présence d'un appel d'outil prime sur le finishReason SAFETY du fixture.
    match &response.choices[0].message.content[0] {
        ContentPart::ToolCall(call) => {
            assert_eq!(call.id, "call-1");
            assert_eq!(call.name, "meteo");
        }
        other => panic!("contenu inattendu : {other:?}"),
    }
    assert_eq!(response.stop_reason, Some(StopReason::ToolCalls));
    Ok(())
}

#[test]
fn stream_emits_stop_then_done() -> Result<(), Box<dyn Error>> {
    let sse = concat!(
        "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"Bon\"}]}}],\"modelVersion\":\"gemini\",\"responseId\":\"r1\"}\n\n",
        "data: {\"candidates\":[{\"content\":{\"parts\":[]},\"finishReason\":\"STOP\"}],\"usageMetadata\":{\"promptTokenCount\":1,\"candidatesTokenCount\":2}}\n\n",
    );
    let mut state = StreamState::new();
    let mut events = Vec::new();
    for chunk in sse.as_bytes().chunks(9) {
        events.extend(
            GeminiProvider::default()
                .parse_stream_chunk(&mut state, bytes::Bytes::copy_from_slice(chunk))?,
        );
    }
    assert!(state.is_finished());
    assert!(matches!(
        events.last(),
        Some(buntu_schema::StreamEvent::Done)
    ));
    assert!(events.iter().any(|event| matches!(
        event,
        buntu_schema::StreamEvent::Stop {
            reason: StopReason::EndTurn
        }
    )));
    Ok(())
}
