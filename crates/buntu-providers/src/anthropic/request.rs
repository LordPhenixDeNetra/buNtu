//! Traduction de la requête canonique vers l'API Anthropic Messages.
//!
//! Références officielles consultées le 2026-11-04 :
//! - Messages API : <https://docs.anthropic.com/en/api/messages>
//! - Streaming : <https://docs.anthropic.com/en/api/messages-streaming>
//! - Outils : <https://docs.anthropic.com/en/docs/build-with-claude/tool-use>

use buntu_schema::{
    ContentPart, GatewayError, ImageSource, Message, Request, ResponseFormat, Role, ToolChoice,
};
use serde_json::{Map, Value, json};

pub const ANTHROPIC_VERSION: &str = "2023-06-01";

pub fn build(
    path: &str,
    default_max_tokens: u32,
    request: &Request,
) -> Result<hyper::Request<bytes::Bytes>, GatewayError> {
    let body = body(request, default_max_tokens)?;
    let bytes = serde_json::to_vec(&body).map_err(|error| internal_error(error.to_string()))?;
    hyper::Request::builder()
        .method(hyper::Method::POST)
        .uri(path)
        .header("content-type", "application/json")
        .header("anthropic-version", ANTHROPIC_VERSION)
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
        message: format!("requête Anthropic : {message}"),
    }
}

fn unsupported(message: String) -> GatewayError {
    GatewayError::InvalidRequest { message }
}

pub fn body(request: &Request, default_max_tokens: u32) -> Result<Value, GatewayError> {
    let mut body = Map::new();
    body.insert("model".to_owned(), Value::String(request.model.clone()));
    body.insert(
        "max_tokens".to_owned(),
        json!(request.max_tokens.unwrap_or(default_max_tokens)),
    );

    let (system, messages) = split_system(&request.messages)?;
    if !system.is_empty() {
        body.insert("system".to_owned(), Value::String(system));
    }
    body.insert("messages".to_owned(), Value::Array(messages));

    if !request.tools.is_empty() {
        let tools: Vec<Value> = request
            .tools
            .iter()
            .map(|tool| {
                json!({
                    "name": tool.name,
                    "description": tool.description,
                    "input_schema": tool.parameters,
                })
            })
            .collect();
        body.insert("tools".to_owned(), Value::Array(tools));
        body.insert("tool_choice".to_owned(), tool_choice(&request.tool_choice));
    }

    if let Some(temperature) = request.temperature {
        body.insert("temperature".to_owned(), json!(temperature));
    }
    if let Some(top_p) = request.top_p {
        body.insert("top_p".to_owned(), json!(top_p));
    }
    if !request.stop.is_empty() {
        body.insert("stop_sequences".to_owned(), json!(request.stop));
    }
    if request.stream {
        body.insert("stream".to_owned(), Value::Bool(true));
    }
    if let Some(format) = &request.response_format {
        if matches!(format, ResponseFormat::JsonObject) {
            // Anthropic n'a pas de mode JSON natif : on le demande par instruction système pour
            // ne pas inventer un champ inexistant.
            let system = body
                .get("system")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .unwrap_or_default();
            body.insert(
                "system".to_owned(),
                Value::String(format!(
                    "{system}\nRéponds uniquement par un objet JSON valide."
                )),
            );
        } else if matches!(format, ResponseFormat::Text) {
            // Comportement par défaut d'Anthropic.
        } else {
            return Err(unsupported(
                "Anthropic ne prend pas en charge `response_format` de type JSON Schema".to_owned(),
            ));
        }
    }

    for (key, value) in &request.extra {
        body.entry(key.clone()).or_insert_with(|| value.clone());
    }

    Ok(Value::Object(body))
}

fn split_system(messages: &[Message]) -> Result<(String, Vec<Value>), GatewayError> {
    let mut system = String::new();
    let mut out = Vec::new();
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
            Role::Tool => {
                let mut results = Vec::new();
                for part in &message.content {
                    let ContentPart::ToolResult(result) = part else {
                        return Err(unsupported(
                            "message « tool » sans résultat d'outil".to_owned(),
                        ));
                    };
                    results.push(json!({
                        "type": "tool_result",
                        "tool_use_id": result.tool_call_id,
                        "content": result.content,
                        "is_error": result.is_error,
                    }));
                }
                out.push(json!({ "role": "user", "content": results }));
            }
            role => {
                let mut blocks = Vec::new();
                for part in &message.content {
                    match part {
                        ContentPart::Text { text } => {
                            blocks.push(json!({ "type": "text", "text": text }));
                        }
                        ContentPart::Image { source, .. } => {
                            blocks.push(json!({
                                "type": "image",
                                "source": image_source(source)?,
                            }));
                        }
                        ContentPart::ToolCall(call) => {
                            blocks.push(json!({
                                "type": "tool_use",
                                "id": call.id,
                                "name": call.name,
                                "input": call.arguments,
                            }));
                        }
                        other => {
                            return Err(unsupported(format!(
                                "contenu non traduit pour Anthropic : {other:?}"
                            )));
                        }
                    }
                }
                out.push(json!({
                    "role": role_str(role),
                    "content": blocks,
                }));
            }
        }
    }
    Ok((system, out))
}

fn role_str(role: Role) -> &'static str {
    match role {
        Role::Assistant => "assistant",
        Role::User | Role::Tool | Role::System | Role::Developer | Role::Function => "user",
    }
}

fn image_source(source: &ImageSource) -> Result<Value, GatewayError> {
    match source {
        ImageSource::Base64 { media_type, data } => Ok(json!({
            "type": "base64",
            "media_type": media_type,
            "data": data,
        })),
        // Anthropic accepte aussi les images par URL (`type: "url"`).
        ImageSource::Url { url } => Ok(json!({ "type": "url", "url": url })),
        ImageSource::File { .. } => Err(unsupported(
            "Anthropic n'accepte pas de fichier image".to_owned(),
        )),
    }
}

fn tool_choice(choice: &ToolChoice) -> Value {
    match choice {
        ToolChoice::Auto => json!({ "type": "auto" }),
        ToolChoice::None => json!({ "type": "none" }),
        ToolChoice::Required => json!({ "type": "any" }),
        ToolChoice::Named(name) => json!({ "type": "tool", "name": name }),
    }
}
