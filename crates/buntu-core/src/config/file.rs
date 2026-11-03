//! Niveau brut : le TOML tel qu'il est écrit, tolérant aux champs inconnus.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config::ConfigError;

pub const DEFAULT_LISTEN: &str = "0.0.0.0:8080";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RawConfig {
    #[serde(default)]
    pub server: RawServer,
    #[serde(default)]
    pub providers: Vec<RawProvider>,
    #[serde(default)]
    pub routes: Vec<RawRoute>,
    #[serde(default)]
    pub hooks: RawHooks,
    #[serde(default)]
    pub timeouts: RawTimeouts,
}

/// Surcharges de timeouts, en millisecondes ; `None` = valeur héritée.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RawTimeouts {
    #[serde(default)]
    pub connect_ms: Option<u64>,
    #[serde(default)]
    pub first_byte_ms: Option<u64>,
    #[serde(default)]
    pub inter_chunk_ms: Option<u64>,
    #[serde(default)]
    pub total_ms: Option<u64>,
}

/// Hooks déclarés, dans l'ordre d'exécution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RawHooks {
    #[serde(default)]
    pub pre: Vec<String>,
    #[serde(default)]
    pub post: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawServer {
    #[serde(default = "default_listen")]
    pub listen: String,
}

impl Default for RawServer {
    fn default() -> Self {
        Self {
            listen: default_listen(),
        }
    }
}

fn default_listen() -> String {
    DEFAULT_LISTEN.to_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawProvider {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub keys: Vec<RawKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawKey {
    /// Référence `env:VAR` (ou valeur littérale pour les tests).
    pub secret: String,
    #[serde(default = "default_weight")]
    pub weight: u32,
}

fn default_weight() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawRoute {
    pub alias: String,
    #[serde(default)]
    pub fallbacks: Vec<String>,
    #[serde(default)]
    pub timeouts: RawTimeouts,
}

/// Lit et désérialise un fichier TOML.
pub fn read(path: &Path) -> Result<RawConfig, ConfigError> {
    let content = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.display().to_string(),
        source,
    })?;
    parse_str(&content, &path.display().to_string())
}

/// Désérialise un TOML déjà en mémoire (utilisé par les tests et `config validate`).
pub fn parse_str(content: &str, origin: &str) -> Result<RawConfig, ConfigError> {
    toml::from_str(content).map_err(|source| ConfigError::Toml {
        path: origin.to_owned(),
        source,
    })
}
