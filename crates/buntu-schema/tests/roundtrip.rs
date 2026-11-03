//! Aller-retour OpenAI ↔ schéma canonique, et robustesse de la sérialisation.
//!
//! Les convertisseurs OpenAI vivent ici, jamais dans le crate : `buntu-schema` ne contient que
//! les types. Les cas fixes sont tirés des exemples de la documentation officielle OpenAI
//! (voir `docs/adr/0002-schema.md`) ; aucun appel réseau n'est fait.

use buntu_schema::{
    ContentPart, ImageDetail, ImageSource, Message, Request, Role, StopReason, Tool, ToolCall,
    ToolChoice, ToolResult, Usage,
};
use proptest::prelude::*;
use serde_json::{Map, Value, json};

// ---------------------------------------------------------------------------------------------
// Convertisseurs OpenAI (spécifiques au test)
// ---------------------------------------------------------------------------------------------

fn role_to_str(role: Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::Developer => "developer",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
        Role::Function => "function",
    }
}

fn detail_to_str(detail: ImageDetail) -> &'static str {
    match detail {
        ImageDetail::Auto => "auto",
        ImageDetail::Low => "low",
        ImageDetail::High => "high",
        ImageDetail::Original => "original",
    }
}

fn image_source_from_url(url: &str) -> ImageSource {
    if let Some(rest) = url.strip_prefix("data:")
        && let Some((media_type, data)) = rest.split_once(";base64,")
    {
        return ImageSource::Base64 {
            media_type: media_type.to_owned(),
            data: data.to_owned(),
        };
    }
    if let Some(file_id) = url.strip_prefix("file:") {
        return ImageSource::File {
            file_id: file_id.to_owned(),
        };
    }
    ImageSource::Url {
        url: url.to_owned(),
    }
}

fn message_to_openai(message: &Message) -> Value {
    let mut object = Map::new();
    object.insert("role".to_owned(), json!(role_to_str(message.role)));
    if let Some(name) = &message.name {
        object.insert("name".to_owned(), json!(name));
    }

    let mut content_parts = Vec::new();
    let mut tool_calls = Vec::new();
    for part in &message.content {
        match part {
            ContentPart::Text { text } => {
                content_parts.push(json!({"type": "text", "text": text}));
            }
            ContentPart::Image { source, detail } => {
                let url = match source {
                    ImageSource::Url { url } => url.clone(),
                    ImageSource::Base64 { media_type, data } => {
                        format!("data:{media_type};base64,{data}")
                    }
                    ImageSource::File { file_id } => format!("file:{file_id}"),
                };
                content_parts.push(json!({
                    "type": "image_url",
                    "image_url": {"url": url, "detail": detail_to_str(*detail)},
                }));
            }
            ContentPart::Audio { data, format } => {
                content_parts.push(json!({
                    "type": "input_audio",
                    "input_audio": {"data": data, "format": format},
                }));
            }
            ContentPart::File { file_id } => {
                content_parts.push(json!({"type": "file", "file": {"file_id": file_id}}));
            }
            ContentPart::Refusal { refusal } => {
                content_parts.push(json!({"type": "refusal", "refusal": refusal}));
            }
            ContentPart::ToolCall(call) => {
                let arguments = match serde_json::to_string(&call.arguments) {
                    Ok(arguments) => arguments,
                    Err(_) => String::from("{}"),
                };
                tool_calls.push(json!({
                    "id": call.id,
                    "type": "function",
                    "function": {"name": call.name, "arguments": arguments},
                }));
            }
            ContentPart::ToolResult(result) => {
                object.insert("role".to_owned(), json!("tool"));
                object.insert("tool_call_id".to_owned(), json!(result.tool_call_id));
                content_parts.push(json!({"type": "text", "text": result.content}));
            }
            ContentPart::Reasoning { .. } | ContentPart::Unknown(_) => {}
        }
    }

    if !content_parts.is_empty() {
        object.insert("content".to_owned(), Value::Array(content_parts));
    }
    if !tool_calls.is_empty() {
        object.insert("tool_calls".to_owned(), Value::Array(tool_calls));
    }
    for (key, value) in &message.extra {
        object.insert(key.clone(), value.clone());
    }
    Value::Object(object)
}

fn tool_to_openai(tool: &Tool) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": tool.name,
            "description": tool.description,
            "parameters": tool.parameters,
        },
    })
}

fn to_openai(request: &Request) -> Value {
    let messages: Vec<Value> = request.messages.iter().map(message_to_openai).collect();
    let mut object = Map::new();
    object.insert("model".to_owned(), json!(request.model));
    object.insert("messages".to_owned(), Value::Array(messages));

    if !request.tools.is_empty() {
        object.insert(
            "tools".to_owned(),
            Value::Array(request.tools.iter().map(tool_to_openai).collect()),
        );
    }
    object.insert(
        "tool_choice".to_owned(),
        json!(match &request.tool_choice {
            ToolChoice::Auto => "auto",
            ToolChoice::None => "none",
            ToolChoice::Required => "required",
            ToolChoice::Named(name) => name.as_str(),
        }),
    );
    if let Some(max_tokens) = request.max_tokens {
        object.insert("max_tokens".to_owned(), json!(max_tokens));
    }
    if let Some(temperature) = request.temperature {
        object.insert("temperature".to_owned(), json!(temperature));
    }
    if let Some(top_p) = request.top_p {
        object.insert("top_p".to_owned(), json!(top_p));
    }
    if !request.stop.is_empty() {
        object.insert("stop".to_owned(), json!(request.stop));
    }
    if request.stream {
        object.insert("stream".to_owned(), json!(true));
    }
    for (key, value) in &request.extra {
        object.insert(key.clone(), value.clone());
    }

    Value::Object(object)
}

const MESSAGE_KEYS: [&str; 5] = ["role", "content", "name", "tool_calls", "tool_call_id"];
const REQUEST_KEYS: [&str; 10] = [
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
];

fn content_part_from_openai(value: &Value) -> Result<ContentPart, serde_json::Error> {
    let kind = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let part = match kind {
        "text" => ContentPart::Text {
            text: string_field(value, "text"),
        },
        "image_url" => {
            let image = value.get("image_url").cloned().unwrap_or(Value::Null);
            let url = string_field(&image, "url");
            let detail = match image.get("detail").and_then(Value::as_str) {
                Some("low") => ImageDetail::Low,
                Some("high") => ImageDetail::High,
                Some("original") => ImageDetail::Original,
                _ => ImageDetail::Auto,
            };
            ContentPart::Image {
                source: image_source_from_url(&url),
                detail,
            }
        }
        "input_audio" => {
            let audio = value.get("input_audio").cloned().unwrap_or(Value::Null);
            ContentPart::Audio {
                data: string_field(&audio, "data"),
                format: string_field(&audio, "format"),
            }
        }
        "file" => {
            let file = value.get("file").cloned().unwrap_or(Value::Null);
            ContentPart::File {
                file_id: string_field(&file, "file_id"),
            }
        }
        "refusal" => ContentPart::Refusal {
            refusal: string_field(value, "refusal"),
        },
        _ => ContentPart::Unknown(value.clone()),
    };
    Ok(part)
}

fn string_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn tool_call_from_openai(value: &Value) -> Result<ToolCall, serde_json::Error> {
    let function = value.get("function").cloned().unwrap_or(Value::Null);
    let arguments = match function.get("arguments") {
        Some(Value::String(arguments)) => serde_json::from_str(arguments)?,
        Some(arguments) => arguments.clone(),
        None => Value::Null,
    };
    Ok(ToolCall {
        id: string_field(value, "id"),
        name: string_field(&function, "name"),
        arguments,
    })
}

fn tool_from_openai(value: &Value) -> Result<Tool, serde_json::Error> {
    let function = value.get("function").cloned().unwrap_or(Value::Null);
    let mut extra = Map::new();
    for (key, entry) in function.as_object().into_iter().flatten() {
        if !matches!(key.as_str(), "name" | "description" | "parameters") {
            extra.insert(key.clone(), entry.clone());
        }
    }
    Ok(Tool {
        name: string_field(&function, "name"),
        description: function
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_owned),
        parameters: function.get("parameters").cloned().unwrap_or(Value::Null),
        extra,
    })
}

fn message_from_openai(value: &Value) -> Result<Message, serde_json::Error> {
    let role: Role = serde_json::from_value(value.get("role").cloned().unwrap_or(Value::Null))?;
    let mut content = Vec::new();

    match value.get("content") {
        Some(Value::String(text)) => {
            if role == Role::Tool {
                content.push(ContentPart::ToolResult(ToolResult {
                    tool_call_id: string_field(value, "tool_call_id"),
                    content: text.clone(),
                    is_error: false,
                }));
            } else {
                content.push(ContentPart::Text { text: text.clone() });
            }
        }
        Some(Value::Array(parts)) => {
            for part in parts {
                let parsed = content_part_from_openai(part)?;
                match parsed {
                    ContentPart::Text { text } if role == Role::Tool => {
                        content.push(ContentPart::ToolResult(ToolResult {
                            tool_call_id: string_field(value, "tool_call_id"),
                            content: text,
                            is_error: false,
                        }));
                    }
                    other => content.push(other),
                }
            }
        }
        _ => {}
    }

    if let Some(Value::Array(calls)) = value.get("tool_calls") {
        for call in calls {
            content.push(ContentPart::ToolCall(tool_call_from_openai(call)?));
        }
    }

    let mut extra = Map::new();
    for (key, entry) in value.as_object().into_iter().flatten() {
        if !MESSAGE_KEYS.contains(&key.as_str()) {
            extra.insert(key.clone(), entry.clone());
        }
    }

    Ok(Message {
        role,
        content,
        name: value.get("name").and_then(Value::as_str).map(str::to_owned),
        extra,
    })
}

fn from_openai(value: &Value) -> Result<Request, serde_json::Error> {
    let mut messages = Vec::new();
    if let Some(Value::Array(items)) = value.get("messages") {
        for item in items {
            messages.push(message_from_openai(item)?);
        }
    }

    let tools = match value.get("tools") {
        Some(Value::Array(items)) => items
            .iter()
            .map(tool_from_openai)
            .collect::<Result<Vec<_>, _>>()?,
        _ => Vec::new(),
    };

    let tool_choice = match value.get("tool_choice") {
        Some(Value::String(choice)) => match choice.as_str() {
            "auto" => ToolChoice::Auto,
            "none" => ToolChoice::None,
            "required" => ToolChoice::Required,
            other => ToolChoice::Named(other.to_owned()),
        },
        Some(Value::Object(_)) => {
            let name = value
                .get("tool_choice")
                .and_then(|choice| choice.get("function"))
                .and_then(|function| function.get("name"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            ToolChoice::Named(name.to_owned())
        }
        _ => ToolChoice::Auto,
    };

    let max_tokens = value
        .get("max_tokens")
        .or_else(|| value.get("max_completion_tokens"))
        .and_then(Value::as_u64)
        .map(|tokens| tokens as u32);
    let temperature = value
        .get("temperature")
        .map(|number| serde_json::from_value::<f32>(number.clone()))
        .transpose()?;
    let top_p = value
        .get("top_p")
        .map(|number| serde_json::from_value::<f32>(number.clone()))
        .transpose()?;
    let stop = match value.get("stop") {
        Some(Value::String(stop)) => vec![stop.clone()],
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.as_str().map(str::to_owned))
            .collect(),
        _ => Vec::new(),
    };
    let stream = value
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut extra = Map::new();
    for (key, entry) in value.as_object().into_iter().flatten() {
        if !REQUEST_KEYS.contains(&key.as_str()) {
            extra.insert(key.clone(), entry.clone());
        }
    }

    Ok(Request {
        model: string_field(value, "model"),
        messages,
        tools,
        tool_choice,
        max_tokens,
        temperature,
        top_p,
        stop,
        stream,
        response_format: None,
        extra,
    })
}

// ---------------------------------------------------------------------------------------------
// Générateurs
// ---------------------------------------------------------------------------------------------

fn arb_text() -> impl Strategy<Value = String> {
    "[ -~]{0,40}"
}

fn arb_role() -> impl Strategy<Value = Role> {
    prop_oneof![Just(Role::System), Just(Role::User), Just(Role::Assistant),]
}

fn arb_message() -> impl Strategy<Value = Message> {
    (
        arb_role(),
        prop::collection::vec(arb_text(), 0..3),
        prop::option::of("[a-z]{1,8}"),
    )
        .prop_map(|(role, texts, name)| Message {
            role,
            content: texts
                .into_iter()
                .map(|text| ContentPart::Text { text })
                .collect(),
            name,
            extra: Map::new(),
        })
}

fn arb_tool() -> impl Strategy<Value = Tool> {
    ("[a-z_]{1,10}", prop::option::of("[ -~]{0,30}")).prop_map(|(name, description)| Tool {
        name,
        description,
        parameters: json!({"type": "object"}),
        extra: Map::new(),
    })
}

fn arb_request() -> impl Strategy<Value = Request> {
    (
        "[a-z0-9/.-]{1,15}",
        prop::collection::vec(arb_message(), 1..4),
        prop::collection::vec(arb_tool(), 0..2),
        prop_oneof![
            Just(ToolChoice::Auto),
            Just(ToolChoice::None),
            Just(ToolChoice::Required),
        ],
        prop::option::of(0_u32..4096),
        prop::option::of(0.0_f32..2.0),
        prop::option::of(0.0_f32..1.0),
        prop::collection::vec("[a-z]{1,5}", 0..2),
        any::<bool>(),
    )
        .prop_map(
            |(
                model,
                messages,
                tools,
                tool_choice,
                max_tokens,
                temperature,
                top_p,
                stop,
                stream,
            )| {
                Request {
                    model,
                    messages,
                    tools,
                    tool_choice,
                    max_tokens,
                    temperature,
                    top_p,
                    stop,
                    stream,
                    response_format: None,
                    extra: Map::new(),
                }
            },
        )
}

// ---------------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    /// OpenAI → canonique → OpenAI : la requête canonique est retrouvée à l'identique.
    #[test]
    fn openai_round_trip(request in arb_request()) {
        let openai = to_openai(&request);
        match from_openai(&openai) {
            Ok(parsed) => prop_assert_eq!(parsed, request),
            Err(error) => prop_assert!(false, "conversion échouée : {error}"),
        }
    }

    /// Canonique → JSON → canonique : la sérialisation ne perd rien.
    #[test]
    fn serde_round_trip(request in arb_request()) {
        match serde_json::to_value(&request) {
            Ok(json) => match serde_json::from_value::<Request>(json) {
                Ok(parsed) => prop_assert_eq!(parsed, request),
                Err(error) => prop_assert!(false, "désérialisation échouée : {error}"),
            },
            Err(error) => prop_assert!(false, "sérialisation échouée : {error}"),
        }
    }
}

#[test]
fn openai_documentation_image_example_round_trips() -> Result<(), serde_json::Error> {
    // Exemple « Image input » de la référence OpenAI Chat Completions.
    let openai: Value = json!({
        "model": "gpt-6-astra",
        "max_tokens": 300,
        "messages": [
            {
                "role": "user",
                "content": [
                    {"type": "text", "text": "What is in this image?"},
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": "https://example.com/image.png",
                            "detail": "high"
                        }
                    }
                ]
            }
        ]
    });

    let request = from_openai(&openai)?;
    let image = request.messages[0]
        .content
        .iter()
        .find_map(|part| match part {
            ContentPart::Image { source, detail } => Some((source.clone(), *detail)),
            _ => None,
        });

    match image {
        Some((ImageSource::Url { url }, ImageDetail::High)) => {
            assert_eq!(url, "https://example.com/image.png");
        }
        other => panic!("image inattendue : {other:?}"),
    }

    let rebuilt = to_openai(&request);
    assert_eq!(
        rebuilt["messages"][0]["content"][1]["image_url"]["detail"],
        "high"
    );
    Ok(())
}

#[test]
fn openai_documentation_tool_example_round_trips() -> Result<(), serde_json::Error> {
    // Exemple « Functions » de la référence OpenAI Chat Completions.
    let openai: Value = json!({
        "model": "gpt-6-astra",
        "messages": [
            {"role": "user", "content": "What is the weather like in Boston today?"},
            {
                "role": "assistant",
                "content": null,
                "tool_calls": [
                    {
                        "id": "call_abc123",
                        "type": "function",
                        "function": {
                            "name": "get_current_weather",
                            "arguments": "{\"location\":\"Boston, MA\"}"
                        }
                    }
                ]
            },
            {
                "role": "tool",
                "tool_call_id": "call_abc123",
                "content": "{\"temperature\": 12}"
            }
        ],
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": "get_current_weather",
                    "description": "Get the current weather in a given location",
                    "parameters": {"type": "object"}
                }
            }
        ],
        "tool_choice": "auto"
    });

    let request = from_openai(&openai)?;
    assert_eq!(request.tools.len(), 1);
    assert_eq!(request.tools[0].name, "get_current_weather");
    assert_eq!(request.tool_choice, ToolChoice::Auto);

    let rebuilt = to_openai(&request);
    assert_eq!(
        rebuilt["messages"][1]["tool_calls"][0]["function"]["name"],
        "get_current_weather"
    );
    assert_eq!(rebuilt["messages"][2]["role"], "tool");
    assert_eq!(rebuilt["messages"][2]["tool_call_id"], "call_abc123");
    Ok(())
}

#[test]
fn openai_unknown_fields_are_preserved_in_extra() -> Result<(), serde_json::Error> {
    let openai: Value = json!({
        "model": "gpt-6-astra",
        "messages": [{"role": "user", "content": "hi", "x_trace": "abc"}],
        "seed": 42
    });

    let request = from_openai(&openai)?;
    assert_eq!(request.extra["seed"], 42);
    assert_eq!(request.messages[0].extra["x_trace"], "abc");

    let rebuilt = to_openai(&request);
    assert_eq!(rebuilt["seed"], 42);
    assert_eq!(rebuilt["messages"][0]["x_trace"], "abc");
    Ok(())
}

#[test]
fn unknown_content_type_is_preserved() -> Result<(), serde_json::Error> {
    let original = json!({"type": "future_modality", "payload": {"x": 1}});
    let part: ContentPart = serde_json::from_value(original.clone())?;
    assert!(matches!(part, ContentPart::Unknown(_)));
    let serialized = serde_json::to_value(&part)?;
    assert_eq!(serialized, original);
    Ok(())
}

#[test]
fn usage_and_stop_reasons_round_trip() -> Result<(), serde_json::Error> {
    let usage = Usage {
        input_tokens: 10,
        output_tokens: 20,
        cached_input_tokens: 3,
        reasoning_tokens: 4,
    };
    assert_eq!(
        serde_json::to_value(usage)?,
        json!({
            "input_tokens": 10,
            "output_tokens": 20,
            "cached_input_tokens": 3,
            "reasoning_tokens": 4,
        })
    );

    let reason = StopReason::Other("custom_reason".to_owned());
    let value = serde_json::to_value(&reason)?;
    assert_eq!(value, json!("custom_reason"));
    assert_eq!(serde_json::from_value::<StopReason>(value)?, reason);

    Ok(())
}
