//! Traduction de la requête canonique vers l'API OpenAI Chat Completions.
//!
//! Références officielles consultées le 2026-11-03 :
//! - OpenAPI officiel (version 2.3.0) : <https://raw.githubusercontent.com/openai/openai-openapi/master/openapi.yaml>
//! - Référence de l'API : <https://platform.openai.com/docs/api-reference/chat/create>

use buntu_schema::{
    ContentPart, GatewayError, ImageSource, Message, Request, ResponseFormat, Role, ToolChoice,
};
use serde_json::{Map, Value, json};

pub fn build(
    path: &str,
    request: &Request,
    include_stream_usage: bool,
) -> Result<hyper::Request<bytes::Bytes>, GatewayError> {
    let body = body(request, include_stream_usage)?;
    let bytes = serde_json::to_vec(&body).map_err(|error| internal_error(error.to_string()))?;
    hyper::Request::builder()
        .method(hyper::Method::POST)
        .uri(path)
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
        message: format!("requête OpenAI : {message}"),
    }
}

fn unsupported(message: String) -> GatewayError {
    GatewayError::InvalidRequest { message }
}

pub fn body(request: &Request, include_stream_usage: bool) -> Result<Value, GatewayError> {
    let mut body = Map::new();
    body.insert("model".to_owned(), Value::String(request.model.clone()));
    body.insert(
        "messages".to_owned(),
        Value::Array(messages(&request.messages)?),
    );

    if !request.tools.is_empty() {
        let tools: Vec<Value> = request
            .tools
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.parameters,
                    },
                })
            })
            .collect();
        body.insert("tools".to_owned(), Value::Array(tools));
        body.insert("tool_choice".to_owned(), tool_choice(&request.tool_choice));
    }

    if let Some(max_tokens) = request.max_tokens {
        body.insert("max_tokens".to_owned(), json!(max_tokens));
    }
    if let Some(temperature) = request.temperature {
        body.insert("temperature".to_owned(), json!(temperature));
    }
    if let Some(top_p) = request.top_p {
        body.insert("top_p".to_owned(), json!(top_p));
    }
    if !request.stop.is_empty() {
        body.insert("stop".to_owned(), json!(request.stop));
    }
    if request.stream {
        body.insert("stream".to_owned(), Value::Bool(true));
        // Nécessaire pour recevoir l'usage en fin de flux (cf. fiche T1.9). Certains serveurs
        // « compatibles OpenAI » refusent ce champ : il est donc optionnel (T1.12).
        if include_stream_usage {
            body.insert(
                "stream_options".to_owned(),
                json!({ "include_usage": true }),
            );
        }
    }
    if let Some(format) = &request.response_format {
        body.insert("response_format".to_owned(), response_format(format));
    }

    for (key, value) in &request.extra {
        body.entry(key.clone()).or_insert_with(|| value.clone());
    }

    Ok(Value::Object(body))
}

fn messages(messages: &[Message]) -> Result<Vec<Value>, GatewayError> {
    let mut out = Vec::new();
    for message in messages {
        if message.role == Role::Tool {
            for part in &message.content {
                let ContentPart::ToolResult(result) = part else {
                    return Err(unsupported(
                        "message « tool » sans résultat d'outil".to_owned(),
                    ));
                };
                let mut entry = Map::new();
                entry.insert("role".to_owned(), Value::String("tool".to_owned()));
                entry.insert(
                    "tool_call_id".to_owned(),
                    Value::String(result.tool_call_id.clone()),
                );
                entry.insert("content".to_owned(), Value::String(result.content.clone()));
                out.push(Value::Object(entry));
            }
            continue;
        }

        let mut entry = Map::new();
        entry.insert(
            "role".to_owned(),
            Value::String(role_str(message.role).to_owned()),
        );

        let mut parts: Vec<Value> = Vec::new();
        let mut tool_calls: Vec<Value> = Vec::new();
        for part in &message.content {
            match part {
                ContentPart::Text { text } => {
                    parts.push(json!({ "type": "text", "text": text }));
                }
                ContentPart::Image { source, .. } => {
                    parts.push(json!({
                        "type": "image_url",
                        "image_url": { "url": image_url(source)? },
                    }));
                }
                ContentPart::ToolCall(call) => {
                    let arguments = serde_json::to_string(&call.arguments)
                        .map_err(|error| internal_error(error.to_string()))?;
                    tool_calls.push(json!({
                        "id": call.id,
                        "type": "function",
                        "function": { "name": call.name, "arguments": arguments },
                    }));
                }
                other => {
                    return Err(unsupported(format!(
                        "contenu non traduit pour OpenAI : {other:?}"
                    )));
                }
            }
        }

        if tool_calls.is_empty() {
            if parts.len() == 1 && parts[0].get("type").and_then(Value::as_str) == Some("text") {
                entry.insert(
                    "content".to_owned(),
                    parts[0].get("text").cloned().unwrap_or(Value::Null),
                );
            } else if parts.is_empty() {
                entry.insert("content".to_owned(), Value::Null);
            } else {
                entry.insert("content".to_owned(), Value::Array(parts));
            }
        } else {
            entry.insert("tool_calls".to_owned(), Value::Array(tool_calls));
            if parts.is_empty() {
                entry.insert("content".to_owned(), Value::Null);
            } else {
                entry.insert("content".to_owned(), Value::Array(parts));
            }
        }

        if let Some(name) = &message.name {
            entry.insert("name".to_owned(), Value::String(name.clone()));
        }
        for (key, value) in &message.extra {
            entry.entry(key.clone()).or_insert_with(|| value.clone());
        }
        out.push(Value::Object(entry));
    }
    Ok(out)
}

fn role_str(role: Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::Developer => "developer",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
        Role::Function => "function",
    }
}

fn image_url(source: &ImageSource) -> Result<String, GatewayError> {
    match source {
        ImageSource::Url { url } => Ok(url.clone()),
        ImageSource::Base64 { media_type, data } => Ok(format!("data:{media_type};base64,{data}")),
        ImageSource::File { .. } => Err(unsupported(
            "l'API Chat Completions n'accepte pas de fichier image".to_owned(),
        )),
    }
}

fn tool_choice(choice: &ToolChoice) -> Value {
    match choice {
        ToolChoice::Auto => Value::String("auto".to_owned()),
        ToolChoice::None => Value::String("none".to_owned()),
        ToolChoice::Required => Value::String("required".to_owned()),
        ToolChoice::Named(name) => json!({
            "type": "function",
            "function": { "name": name },
        }),
    }
}

fn response_format(format: &ResponseFormat) -> Value {
    match format {
        ResponseFormat::Text => json!({ "type": "text" }),
        ResponseFormat::JsonObject => json!({ "type": "json_object" }),
        ResponseFormat::JsonSchema { name, schema } => json!({
            "type": "json_schema",
            "json_schema": { "name": name, "schema": schema },
        }),
    }
}
