//! Traduction de la requête canonique vers l'API Gemini (AI Studio, `generateContent`).
//!
//! Références officielles consultées le 2026-11-04 :
//! - API `generateContent` : <https://ai.google.dev/api/generate-content>
//! - Outils : <https://ai.google.dev/gemini-api/docs/function-calling>
//! - Streaming : <https://ai.google.dev/gemini-api/docs/streaming>

use buntu_schema::{
    ContentPart, GatewayError, ImageSource, Message, Request, ResponseFormat, Role, ToolChoice,
};
use serde_json::{Map, Value, json};

pub const DEFAULT_BASE: &str = "/v1beta/models";

/// Construit l'URL de génération pour un modèle donné.
pub fn endpoint(base: &str, model: &str, stream: bool) -> String {
    let base = if base.is_empty() { DEFAULT_BASE } else { base };
    if stream {
        format!("{base}/{model}:streamGenerateContent?alt=sse")
    } else {
        format!("{base}/{model}:generateContent")
    }
}

pub fn build(base: &str, request: &Request) -> Result<hyper::Request<bytes::Bytes>, GatewayError> {
    let body = body(request)?;
    let bytes = serde_json::to_vec(&body).map_err(|error| internal_error(error.to_string()))?;
    hyper::Request::builder()
        .method(hyper::Method::POST)
        .uri(endpoint(base, &request.model, request.stream))
        .header("content-type", "application/json")
        .header(
            "accept",
            if request.stream {
                "text/event-stream"
            } else {
                "application/json"
            },
        )
        .body(bytes::Bytes::from(bytes))
        .map_err(|error| internal_error(error.to_string()))
}

fn internal_error(message: String) -> GatewayError {
    GatewayError::Internal {
        message: format!("requête Gemini : {message}"),
    }
}

fn unsupported(message: String) -> GatewayError {
    GatewayError::InvalidRequest { message }
}

pub fn body(request: &Request) -> Result<Value, GatewayError> {
    let mut body = Map::new();

    let (system, contents) = split_messages(&request.messages)?;
    if !system.is_empty() {
        body.insert(
            "systemInstruction".to_owned(),
            json!({ "parts": [{ "text": system }] }),
        );
    }
    if contents.is_empty() {
        return Err(unsupported(
            "Gemini exige au moins un message dans `contents`".to_owned(),
        ));
    }
    body.insert("contents".to_owned(), Value::Array(contents));

    if !request.tools.is_empty() {
        let declarations: Vec<Value> = request
            .tools
            .iter()
            .map(|tool| {
                json!({
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.parameters,
                })
            })
            .collect();
        body.insert(
            "tools".to_owned(),
            json!([{ "functionDeclarations": declarations }]),
        );
        body.insert("toolConfig".to_owned(), tool_config(&request.tool_choice));
    }

    let mut generation = Map::new();
    if let Some(max_tokens) = request.max_tokens {
        generation.insert("maxOutputTokens".to_owned(), json!(max_tokens));
    }
    if let Some(temperature) = request.temperature {
        generation.insert("temperature".to_owned(), json!(temperature));
    }
    if let Some(top_p) = request.top_p {
        generation.insert("topP".to_owned(), json!(top_p));
    }
    if !request.stop.is_empty() {
        generation.insert("stopSequences".to_owned(), json!(request.stop));
    }
    if let Some(format) = &request.response_format {
        match format {
            ResponseFormat::Text => {}
            ResponseFormat::JsonObject => {
                generation.insert("responseMimeType".to_owned(), json!("application/json"));
            }
            ResponseFormat::JsonSchema { schema, .. } => {
                generation.insert("responseMimeType".to_owned(), json!("application/json"));
                generation.insert("responseSchema".to_owned(), schema.clone());
            }
        }
    }
    if !generation.is_empty() {
        body.insert("generationConfig".to_owned(), Value::Object(generation));
    }

    for (key, value) in &request.extra {
        body.entry(key.clone()).or_insert_with(|| value.clone());
    }

    Ok(Value::Object(body))
}

fn split_messages(messages: &[Message]) -> Result<(String, Vec<Value>), GatewayError> {
    let mut system = String::new();
    let mut contents = Vec::new();
    for message in messages {
        match message.role {
            Role::System | Role::Developer => {
                for part in &message.content {
                    if let ContentPart::Text { text } = part {
                        if !system.is_empty() {
                            system.push('\n');
                        }
                        system.push_str(text);
                    }
                }
            }
            role => {
                let mut parts = Vec::new();
                for part in &message.content {
                    match part {
                        ContentPart::Text { text } => parts.push(json!({ "text": text })),
                        ContentPart::Image { source, .. } => {
                            let ImageSource::Base64 { media_type, data } = source else {
                                return Err(unsupported(
                                    "Gemini attend une image encodée en base64".to_owned(),
                                ));
                            };
                            parts.push(json!({
                                "inlineData": { "mimeType": media_type, "data": data },
                            }));
                        }
                        ContentPart::ToolCall(call) => parts.push(json!({
                            "functionCall": { "name": call.name, "args": call.arguments },
                        })),
                        ContentPart::ToolResult(result) => parts.push(json!({
                            // Le canonique ne transporte pas le nom de l'outil dans le résultat :
                            // `tool_call_id` sert de nom (limite documentée).
                            "functionResponse": {
                                "name": result.tool_call_id,
                                "response": { "content": result.content },
                            },
                        })),
                        other => {
                            return Err(unsupported(format!(
                                "contenu non traduit pour Gemini : {other:?}"
                            )));
                        }
                    }
                }
                contents.push(json!({
                    "role": if role == Role::Assistant { "model" } else { "user" },
                    "parts": parts,
                }));
            }
        }
    }
    Ok((system, contents))
}

fn tool_config(choice: &ToolChoice) -> Value {
    let mode = match choice {
        ToolChoice::Auto => "AUTO",
        ToolChoice::None => "NONE",
        ToolChoice::Required | ToolChoice::Named(_) => "ANY",
    };
    let mut config = Map::new();
    config.insert("mode".to_owned(), json!(mode));
    if let ToolChoice::Named(name) = choice {
        config.insert("allowedFunctionNames".to_owned(), json!([name]));
    }
    json!({ "functionCallingConfig": config })
}
