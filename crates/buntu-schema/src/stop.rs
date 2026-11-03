//! Raison d'arrêt d'une génération.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Pourquoi le modèle a arrêté de générer.
///
/// Le nom sérialisé suit le vocabulaire OpenAI (`stop`, `length`, `tool_calls`,
/// `content_filter`) ; Anthropic et Gemini seront traduits vers ces valeurs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    EndTurn,
    StopSequence,
    MaxTokens,
    ToolCalls,
    ContentFilter,
    Pause,
    Refusal,
    Other(String),
}

impl StopReason {
    fn as_str(&self) -> &str {
        match self {
            StopReason::EndTurn => "end_turn",
            StopReason::StopSequence => "stop_sequence",
            StopReason::MaxTokens => "max_tokens",
            StopReason::ToolCalls => "tool_calls",
            StopReason::ContentFilter => "content_filter",
            StopReason::Pause => "pause",
            StopReason::Refusal => "refusal",
            StopReason::Other(value) => value,
        }
    }
}

impl Serialize for StopReason {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for StopReason {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let reason = match value.as_str() {
            "end_turn" => StopReason::EndTurn,
            "stop_sequence" => StopReason::StopSequence,
            "max_tokens" => StopReason::MaxTokens,
            "tool_calls" => StopReason::ToolCalls,
            "content_filter" => StopReason::ContentFilter,
            "pause" => StopReason::Pause,
            "refusal" => StopReason::Refusal,
            other => StopReason::Other(other.to_owned()),
        };
        Ok(reason)
    }
}
