//! Tests unitaires du fournisseur compatible OpenAI.

use std::error::Error;

use buntu_providers::compatible::{Compatible, OpenAiCompatibleProvider, Quirks};
use buntu_providers::provider::Provider;
use buntu_schema::{ContentPart, Message, Request, Role, Tool};
use serde_json::{Map, Value, json};

fn request() -> Request {
    Request {
        model: "llama3.2".to_owned(),
        messages: vec![Message {
            role: Role::User,
            content: vec![ContentPart::Text {
                text: "Bonjour".to_owned(),
            }],
            name: None,
            extra: Map::new(),
        }],
        tools: Vec::new(),
        tool_choice: Default::default(),
        max_tokens: None,
        temperature: None,
        top_p: None,
        stop: Vec::new(),
        stream: true,
        response_format: None,
        extra: Map::new(),
    }
}

#[test]
fn ollama_preset_omits_stream_options() -> Result<(), Box<dyn Error>> {
    let provider = OpenAiCompatibleProvider::preset(Compatible::Ollama);
    assert_eq!(provider.preset_name(), Some(Compatible::Ollama));
    let built = provider.build_request(&request())?;
    let body: Value = serde_json::from_slice(built.body())?;
    assert!(body.get("stream_options").is_none());
    assert_eq!(body["stream"], true);
    Ok(())
}

#[test]
fn default_keeps_stream_options_for_openai_compatible_servers() -> Result<(), Box<dyn Error>> {
    let provider = OpenAiCompatibleProvider::default();
    let built = provider.build_request(&request())?;
    let body: Value = serde_json::from_slice(built.body())?;
    assert_eq!(body["stream_options"]["include_usage"], true);
    Ok(())
}

#[test]
fn base_url_and_headers_are_applied() -> Result<(), Box<dyn Error>> {
    let provider = OpenAiCompatibleProvider::preset(Compatible::Groq)
        .with_base_url("https://api.groq.com/openai")
        .with_header("x-test", "1");
    let built = provider.build_request(&request())?;
    assert_eq!(
        built.uri().to_string(),
        "https://api.groq.com/openai/v1/chat/completions"
    );
    assert_eq!(
        built.headers().get("x-test").and_then(|v| v.to_str().ok()),
        Some("1")
    );
    Ok(())
}

#[test]
fn parallel_tool_quirk_disables_parallel_calls() -> Result<(), Box<dyn Error>> {
    let mut request = request();
    request.tools.push(Tool {
        name: "meteo".to_owned(),
        description: None,
        parameters: json!({"type": "object"}),
        extra: Map::new(),
    });
    let provider = OpenAiCompatibleProvider::default().with_quirks(Quirks {
        stream_usage: true,
        parallel_tools: false,
    });
    assert!(!provider.quirks().parallel_tools);
    let built = provider.build_request(&request)?;
    let body: Value = serde_json::from_slice(built.body())?;
    assert_eq!(body["parallel_tool_calls"], false);
    Ok(())
}

#[test]
fn name_is_openai_compatible() {
    assert_eq!(
        OpenAiCompatibleProvider::default().name(),
        "openai-compatible"
    );
}
