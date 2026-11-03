//! Outils : définitions, appels et résultats.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Définition d'un outil exposé au modèle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub parameters: Value,
    /// Champs fournisseur non modélisés, conservés tels quels.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Un appel d'outil demandé par le modèle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    /// Arguments déjà décodés (les fournisseurs les envoient en chaîne JSON).
    #[serde(default)]
    pub arguments: Value,
}

/// Le résultat d'un appel d'outil, renvoyé au modèle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub content: String,
    #[serde(default)]
    pub is_error: bool,
}
