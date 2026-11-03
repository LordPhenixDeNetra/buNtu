//! Événements d'un flux de réponse, indépendants du fournisseur.

use serde::{Deserialize, Serialize};

use crate::content::ContentPart;
use crate::stop::StopReason;
use crate::usage::Usage;

/// Un événement du flux canonique.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// Début du flux : identifiant de réponse et modèle effectif.
    Start { id: String, model: String },
    /// Un morceau de contenu ajouté au bloc `index`.
    ContentDelta { index: u32, part: ContentPart },
    /// Un fragment d'appel d'outil : les arguments arrivent par morceaux.
    ToolCallDelta {
        index: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        arguments_delta: Option<String>,
    },
    /// Compteurs d'usage, généralement en fin de flux.
    Usage { usage: Usage },
    /// Raison d'arrêt.
    Stop { reason: StopReason },
    /// Fin normale du flux.
    Done,
}
