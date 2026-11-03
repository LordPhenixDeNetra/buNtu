//! Requête canonique et paramètres de génération.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use crate::message::Message;
use crate::tool::Tool;

/// Choix d'outil : laisse le modèle décider, l'interdit, l'impose, ou nomme un outil.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ToolChoice {
    #[default]
    Auto,
    None,
    Required,
    Named(String),
}

/// Format de réponse demandé.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseFormat {
    Text,
    JsonObject,
    JsonSchema { name: String, schema: Value },
}

/// Une requête de complétion, indépendante du fournisseur.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub tools: Vec<Tool>,
    #[serde(default)]
    pub tool_choice: ToolChoice,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(default)]
    pub stop: Vec<String>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    /// Champs fournisseur non modélisés, conservés tels quels.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Serialize for ToolChoice {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = match self {
            ToolChoice::Auto => "auto",
            ToolChoice::None => "none",
            ToolChoice::Required => "required",
            ToolChoice::Named(name) => name.as_str(),
        };
        serializer.serialize_str(value)
    }
}

impl<'de> Deserialize<'de> for ToolChoice {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let choice = match value.as_str() {
            "auto" => ToolChoice::Auto,
            "none" => ToolChoice::None,
            "required" => ToolChoice::Required,
            other => ToolChoice::Named(other.to_owned()),
        };
        Ok(choice)
    }
}
