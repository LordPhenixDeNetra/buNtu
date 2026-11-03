//! Messages et rôles du schéma canonique.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::content::ContentPart;

/// Rôle de l'auteur d'un message.
///
/// `Developer` couvre le rôle `developer` d'OpenAI (successeur de `system` sur les modèles
/// récents) ; `Tool` porte les résultats d'outils au format OpenAI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    Developer,
    User,
    Assistant,
    Tool,
    Function,
}

/// Un message de la conversation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    #[serde(default)]
    pub content: Vec<ContentPart>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Champs fournisseur non modélisés, conservés tels quels.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
