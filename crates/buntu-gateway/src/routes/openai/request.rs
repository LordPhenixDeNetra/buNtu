//! Désérialisation d'une requête OpenAI (`/v1/chat/completions`) vers le canonique.
//!
//! Référence : <https://platform.openai.com/docs/api-reference/chat/create> (consultée le
//! 2026-11-04). Les champs inconnus sont conservés dans `extra` (passthrough).

use buntu_schema::{
    ContentPart, GatewayError, ImageDetail, ImageSource, Message, Request, ResponseFormat, Role,
    Tool, ToolChoice,
};
use serde_json::{Map, Value};

const KNOWN_FIELDS: &[&str] = &[
    "model",
    "messages",
    "tools",
    "tool_choice",
    "max_tokens",
    "max_completion_tokens",
    "temperature",
    "top_p",
    "stop",
    "stream",
    "response_format",
];

pub fn parse(body: &Value) -> Result<Request, GatewayError> {
    let model = string_field(body, "model")?;
    let messages = body
        .get("messages")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("`messages` est requis"))?;
    let mut canonical_messages = Vec::with_capacity(messages.len());
    for message in messages {
        canonical_messages.extend(parse_message(message)?);
    }

    let mut tools = Vec::new();
    if let Some(values) = body.get("tools").and_then(Value::as_array) {
        for value in values {
            tools.push(parse_tool(value)?);
        }
    }

    let extra = body
        .as_object()
        .map(|object| {
            object
                .iter()
                .filter(|(key, _)| !KNOWN_FIELDS.contains(&key.as_str()))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect::<Map<String, Value>>()
        })
        .unwrap_or_default();

    Ok(Request {
        model,
        messages: canonical_messages,
        tools,
        tool_choice: body
            .get("tool_choice")
            .map(parse_tool_choice)
            .transpose()?
            .unwrap_or_default(),
        max_tokens: body
            .get("max_tokens")
            .or_else(|| body.get("max_completion_tokens"))
            .and_then(Value::as_u64)
            .map(|value| value as u32),
        temperature: body
            .get("temperature")
            .and_then(Value::as_f64)
            .map(|value| value as f32),
        top_p: body
            .get("top_p")
            .and_then(Value::as_f64)
            .map(|value| value as f32),
        stop: parse_stop(body.get("stop"))?,
        stream: body.get("stream").and_then(Value::as_bool).unwrap_or(false),
        response_format: body
            .get("response_format")
            .map(parse_response_format)
            .transpose()?,
        extra,
    })
}

fn parse_message(value: &Value) -> Result<Vec<Message>, GatewayError> {
    let role = match value.get("role").and_then(Value::as_str) {
        Some("system") => Role::System,
        Some("developer") => Role::Developer,
        Some("user") => Role::User,
        Some("assistant") => Role::Assistant,
        Some("tool") | Some("function") => Role::Tool,
        other => return Err(invalid(format!("rôle inconnu : {other:?}"))),
    };

    if role == Role::Tool {
        let tool_call_id = string_field(value, "tool_call_id")?;
        let content = value
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        return Ok(vec![Message {
            role,
            content: vec![ContentPart::ToolResult(buntu_schema::ToolResult {
                tool_call_id,
                content,
                is_error: false,
            })],
            name: value.get("name").and_then(Value::as_str).map(str::to_owned),
            extra: Map::new(),
        }]);
    }

    let mut content = parse_content(value.get("content"))?;
    if let Some(calls) = value.get("tool_calls").and_then(Value::as_array) {
        for call in calls {
            let function = call.get("function").cloned().unwrap_or(Value::Null);
            let arguments = function
                .get("arguments")
                .and_then(Value::as_str)
                .map(|raw| {
                    serde_json::from_str::<Value>(raw).unwrap_or(Value::String(raw.to_owned()))
                })
                .unwrap_or(Value::Null);
            content.push(ContentPart::ToolCall(buntu_schema::ToolCall {
                id: call
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                name: function
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                arguments,
            }));
        }
    }

    Ok(vec![Message {
        role,
        content,
        name: value.get("name").and_then(Value::as_str).map(str::to_owned),
        extra: Map::new(),
    }])
}

fn parse_content(value: Option<&Value>) -> Result<Vec<ContentPart>, GatewayError> {
    match value {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::String(text)) if text.is_empty() => Ok(Vec::new()),
        Some(Value::String(text)) => Ok(vec![ContentPart::Text { text: text.clone() }]),
        Some(Value::Array(parts)) => {
            let mut content = Vec::with_capacity(parts.len());
            for part in parts {
                match part.get("type").and_then(Value::as_str) {
                    Some("text") => content.push(ContentPart::Text {
                        text: text_of(part.get("text")),
                    }),
                    Some("image_url") => {
                        let url = text_of(part.get("image_url").and_then(|image| image.get("url")));
                        let detail = part
                            .get("image_url")
                            .and_then(|image| image.get("detail"))
                            .and_then(Value::as_str)
                            .map(|detail| match detail {
                                "low" => ImageDetail::Low,
                                "high" => ImageDetail::High,
                                "original" => ImageDetail::Original,
                                _ => ImageDetail::Auto,
                            })
                            .unwrap_or_default();
                        content.push(ContentPart::Image {
                            source: parse_image_source(&url),
                            detail,
                        });
                    }
                    other => {
                        return Err(invalid(format!(
                            "type de contenu non pris en charge : {other:?}"
                        )));
                    }
                }
            }
            Ok(content)
        }
        Some(other) => Err(invalid(format!("contenu de message invalide : {other}"))),
    }
}

fn parse_image_source(url: &str) -> ImageSource {
    if let Some(rest) = url.strip_prefix("data:")
        && let Some((metadata, data)) = rest.split_once(',')
        && let Some(media_type) = metadata.strip_suffix(";base64")
    {
        return ImageSource::Base64 {
            media_type: media_type.to_owned(),
            data: data.to_owned(),
        };
    }
    ImageSource::Url {
        url: url.to_owned(),
    }
}

fn parse_tool(value: &Value) -> Result<Tool, GatewayError> {
    if value.get("type").and_then(Value::as_str) != Some("function") {
        return Err(invalid("seuls les outils `function` sont acceptés"));
    }
    let function = value
        .get("function")
        .ok_or_else(|| invalid("outil sans `function`"))?;
    Ok(Tool {
        name: string_field(function, "name")?,
        description: function
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_owned),
        parameters: function
            .get("parameters")
            .cloned()
            .unwrap_or(Value::Object(Map::new())),
        extra: Map::new(),
    })
}

fn parse_tool_choice(value: &Value) -> Result<ToolChoice, GatewayError> {
    match value {
        Value::String(choice) => match choice.as_str() {
            "auto" => Ok(ToolChoice::Auto),
            "none" => Ok(ToolChoice::None),
            "required" => Ok(ToolChoice::Required),
            other => Err(invalid(format!("tool_choice inconnu : {other}"))),
        },
        Value::Object(_) => {
            let name = value
                .get("function")
                .and_then(|function| function.get("name"))
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("tool_choice nommé sans `function.name`"))?;
            Ok(ToolChoice::Named(name.to_owned()))
        }
        other => Err(invalid(format!("tool_choice invalide : {other}"))),
    }
}

fn parse_stop(value: Option<&Value>) -> Result<Vec<String>, GatewayError> {
    match value {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::String(text)) => Ok(vec![text.clone()]),
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| invalid("`stop` doit contenir des chaînes"))
            })
            .collect(),
        Some(other) => Err(invalid(format!("`stop` invalide : {other}"))),
    }
}

fn parse_response_format(value: &Value) -> Result<ResponseFormat, GatewayError> {
    match value.get("type").and_then(Value::as_str) {
        Some("text") => Ok(ResponseFormat::Text),
        Some("json_object") => Ok(ResponseFormat::JsonObject),
        Some("json_schema") => {
            let schema = value
                .get("json_schema")
                .ok_or_else(|| invalid("json_schema sans corps"))?;
            Ok(ResponseFormat::JsonSchema {
                name: schema
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("schema")
                    .to_owned(),
                schema: schema
                    .get("schema")
                    .cloned()
                    .unwrap_or(Value::Object(Map::new())),
            })
        }
        other => Err(invalid(format!("response_format inconnu : {other:?}"))),
    }
}

fn string_field(value: &Value, field: &str) -> Result<String, GatewayError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| invalid(format!("`{field}` est requis")))
}

fn text_of(value: Option<&Value>) -> String {
    value.and_then(Value::as_str).unwrap_or_default().to_owned()
}

fn invalid(message: impl Into<String>) -> GatewayError {
    GatewayError::InvalidRequest {
        message: message.into(),
    }
}
