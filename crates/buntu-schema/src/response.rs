//! Réponse canonique complète (hors flux), utilisée par le pipeline et les hooks.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::message::Message;
use crate::stop::StopReason;
use crate::usage::Usage;

/// Une réponse complète, indépendante du fournisseur.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response {
    pub id: String,
    pub model: String,
    #[serde(default)]
    pub choices: Vec<Choice>,
    #[serde(default)]
    pub usage: Usage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<StopReason>,
    /// Champs fournisseur non modélisés, conservés tels quels.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Une alternative de la réponse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
}
