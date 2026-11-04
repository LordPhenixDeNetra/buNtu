//! Requêtes et réponses d'embeddings canoniques (T1.13).
//!
//! Les vecteurs sont des `f32` (format natif des fournisseurs) ; les coûts, eux, seront
//! calculés en `rust_decimal` par la gouvernance.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Demande d'embeddings, indépendante du fournisseur.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingsRequest {
    pub model: String,
    /// Une ou plusieurs entrées.
    pub input: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<String>,
    /// Champs fournisseur non modélisés, conservés.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Un vecteur produit pour une entrée.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Embedding {
    pub index: u32,
    pub embedding: Vec<f32>,
    #[serde(default)]
    pub object: String,
}

/// Usage d'une requête d'embeddings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingUsage {
    #[serde(default)]
    pub prompt_tokens: u32,
    #[serde(default)]
    pub total_tokens: u32,
}

/// Réponse d'embeddings canonique.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingsResponse {
    pub model: String,
    #[serde(default)]
    pub data: Vec<Embedding>,
    #[serde(default)]
    pub usage: EmbeddingUsage,
}
