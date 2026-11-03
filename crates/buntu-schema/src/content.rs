//! Contenus d'un message : texte, image, audio, fichier, appels et résultats d'outils,
//! réflexion, refus.

use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::tool::{ToolCall, ToolResult};

/// Niveau de détail demandé pour une image (vocabulaire OpenAI ; `Auto` par défaut).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageDetail {
    #[default]
    Auto,
    Low,
    High,
    Original,
}

/// Origine d'une image : URL, base64, ou fichier déjà téléversé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    Url { url: String },
    Base64 { media_type: String, data: String },
    File { file_id: String },
}

/// Une partie du contenu d'un message.
///
/// La sérialisation est manuelle afin de conserver intact un type de contenu inconnu
/// (`Unknown`) au lieu de le perdre : un fournisseur peut introduire une modalité nouvelle.
#[derive(Debug, Clone, PartialEq)]
pub enum ContentPart {
    Text {
        text: String,
    },
    Image {
        source: ImageSource,
        detail: ImageDetail,
    },
    Audio {
        data: String,
        format: String,
    },
    File {
        file_id: String,
    },
    ToolCall(ToolCall),
    ToolResult(ToolResult),
    Reasoning {
        text: String,
        signature: Option<String>,
    },
    Refusal {
        refusal: String,
    },
    Unknown(Value),
}

#[derive(Deserialize)]
struct TextPayload {
    text: String,
}

#[derive(Deserialize)]
struct ImagePayload {
    source: ImageSource,
    #[serde(default)]
    detail: ImageDetail,
}

#[derive(Deserialize)]
struct AudioPayload {
    data: String,
    format: String,
}

#[derive(Deserialize)]
struct FilePayload {
    file_id: String,
}

#[derive(Deserialize)]
struct ReasoningPayload {
    text: String,
    #[serde(default)]
    signature: Option<String>,
}

#[derive(Deserialize)]
struct RefusalPayload {
    refusal: String,
}

impl Serialize for ContentPart {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ContentPart::Text { text } => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("type", "text")?;
                map.serialize_entry("text", text)?;
                map.end()
            }
            ContentPart::Image { source, detail } => {
                let mut map = serializer.serialize_map(Some(3))?;
                map.serialize_entry("type", "image")?;
                map.serialize_entry("source", source)?;
                map.serialize_entry("detail", detail)?;
                map.end()
            }
            ContentPart::Audio { data, format } => {
                let mut map = serializer.serialize_map(Some(3))?;
                map.serialize_entry("type", "audio")?;
                map.serialize_entry("data", data)?;
                map.serialize_entry("format", format)?;
                map.end()
            }
            ContentPart::File { file_id } => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("type", "file")?;
                map.serialize_entry("file_id", file_id)?;
                map.end()
            }
            ContentPart::ToolCall(call) => {
                let mut map = serializer.serialize_map(Some(4))?;
                map.serialize_entry("type", "tool_call")?;
                map.serialize_entry("id", &call.id)?;
                map.serialize_entry("name", &call.name)?;
                map.serialize_entry("arguments", &call.arguments)?;
                map.end()
            }
            ContentPart::ToolResult(result) => {
                let mut map = serializer.serialize_map(Some(4))?;
                map.serialize_entry("type", "tool_result")?;
                map.serialize_entry("tool_call_id", &result.tool_call_id)?;
                map.serialize_entry("content", &result.content)?;
                map.serialize_entry("is_error", &result.is_error)?;
                map.end()
            }
            ContentPart::Reasoning { text, signature } => {
                let count = if signature.is_some() { 3 } else { 2 };
                let mut map = serializer.serialize_map(Some(count))?;
                map.serialize_entry("type", "reasoning")?;
                map.serialize_entry("text", text)?;
                if let Some(signature) = signature {
                    map.serialize_entry("signature", signature)?;
                }
                map.end()
            }
            ContentPart::Refusal { refusal } => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("type", "refusal")?;
                map.serialize_entry("refusal", refusal)?;
                map.end()
            }
            ContentPart::Unknown(value) => value.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ContentPart {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let kind = value.get("type").and_then(Value::as_str).map(str::to_owned);

        let part = match kind.as_deref() {
            Some("text") => ContentPart::Text {
                text: from_value::<TextPayload, D>(value)?.text,
            },
            Some("image") => {
                let payload = from_value::<ImagePayload, D>(value)?;
                ContentPart::Image {
                    source: payload.source,
                    detail: payload.detail,
                }
            }
            Some("audio") => {
                let payload = from_value::<AudioPayload, D>(value)?;
                ContentPart::Audio {
                    data: payload.data,
                    format: payload.format,
                }
            }
            Some("file") => {
                let payload = from_value::<FilePayload, D>(value)?;
                ContentPart::File {
                    file_id: payload.file_id,
                }
            }
            Some("tool_call") => ContentPart::ToolCall(from_value::<ToolCall, D>(value)?),
            Some("tool_result") => ContentPart::ToolResult(from_value::<ToolResult, D>(value)?),
            Some("reasoning") => {
                let payload = from_value::<ReasoningPayload, D>(value)?;
                ContentPart::Reasoning {
                    text: payload.text,
                    signature: payload.signature,
                }
            }
            Some("refusal") => {
                let payload = from_value::<RefusalPayload, D>(value)?;
                ContentPart::Refusal {
                    refusal: payload.refusal,
                }
            }
            _ => ContentPart::Unknown(value),
        };

        Ok(part)
    }
}

fn from_value<'de, T, D>(value: Value) -> Result<T, D::Error>
where
    T: serde::de::DeserializeOwned,
    D: Deserializer<'de>,
{
    serde_json::from_value(value).map_err(serde::de::Error::custom)
}
