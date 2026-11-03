//! Compteurs d'usage d'une requête.

use serde::{Deserialize, Serialize};

/// Tokens consommés par une requête.
///
/// Ce sont des compteurs entiers, jamais des montants : les coûts seront calculés en
/// `rust_decimal` par `buntu-governance` (cahier §6.3).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub input_tokens: u32,
    #[serde(default)]
    pub output_tokens: u32,
    /// Tokens d'entrée servis depuis le cache de prompt du fournisseur.
    #[serde(default)]
    pub cached_input_tokens: u32,
    /// Tokens de réflexion facturés en sortie.
    #[serde(default)]
    pub reasoning_tokens: u32,
}
