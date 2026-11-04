//! Niveau brut : le TOML tel qu'il est écrit, tolérant aux champs inconnus.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config::ConfigError;

pub const DEFAULT_LISTEN: &str = "0.0.0.0:8080";

#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RawConfig {
    /// Paramètres du serveur HTTP.
    #[serde(default)]
    pub server: RawServer,
    /// Fournisseurs de modèles configurés.
    #[serde(default)]
    pub providers: Vec<RawProvider>,
    /// Alias de modèles et chaînes de repli.
    #[serde(default)]
    pub routes: Vec<RawRoute>,
    /// Hooks exécutés avant et après l'appel fournisseur.
    #[serde(default)]
    pub hooks: RawHooks,
    /// Timeouts par défaut, en millisecondes.
    #[serde(default)]
    pub timeouts: RawTimeouts,
}

/// Surcharges de timeouts, en millisecondes ; `None` = valeur héritée.
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RawTimeouts {
    /// Délai maximal d'établissement de la connexion amont.
    #[serde(default)]
    pub connect_ms: Option<u64>,
    /// Délai maximal avant le premier octet de la réponse amont.
    #[serde(default)]
    pub first_byte_ms: Option<u64>,
    /// Délai maximal entre deux morceaux d'un flux amont.
    #[serde(default)]
    pub inter_chunk_ms: Option<u64>,
    /// Délai maximal total d'une requête.
    #[serde(default)]
    pub total_ms: Option<u64>,
}

/// Hooks déclarés, dans l'ordre d'exécution.
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RawHooks {
    /// Hooks exécutés avant l'appel fournisseur.
    #[serde(default)]
    pub pre: Vec<String>,
    /// Hooks exécutés après l'appel fournisseur.
    #[serde(default)]
    pub post: Vec<String>,
}

/// Paramètres du serveur HTTP.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RawServer {
    /// Adresse d'écoute (`0.0.0.0:8080` par défaut).
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

/// Un fournisseur de modèles.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RawProvider {
    /// Identifiant unique, utilisé dans les cibles (`openai/gpt-4o`).
    pub id: String,
    /// Type de fournisseur : `openai`, `anthropic`, `gemini`, `compatible`, `mock`…
    #[serde(rename = "type")]
    #[schemars(rename = "type")]
    pub kind: String,
    /// URL de base de l'API du fournisseur (ex. `http://127.0.0.1:9000`).
    #[serde(default)]
    pub base_url: Option<String>,
    /// Clés API, sélectionnées par poids décroissant.
    #[serde(default)]
    pub keys: Vec<RawKey>,
}

/// Une clé API de fournisseur.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RawKey {
    /// Référence `env:VAR` (recommandé en production) ou valeur littérale (tests).
    pub secret: String,
    /// Poids de sélection ; une clé plus lourde est choisie plus souvent.
    #[serde(default = "default_weight")]
    pub weight: u32,
}

fn default_weight() -> u32 {
    1
}

/// Un alias de modèle et sa chaîne de repli.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RawRoute {
    /// Nom exposé par l'API (`model: "chat"`).
    pub alias: String,
    /// Cibles essayées dans l'ordre : `fournisseur/modèle`.
    #[serde(default)]
    pub fallbacks: Vec<String>,
    /// Surcharges de timeouts pour cet alias.
    #[serde(default)]
    pub timeouts: RawTimeouts,
}

/// Schéma JSON de la configuration, généré depuis les types Rust (T1.27).
pub fn json_schema() -> serde_json::Value {
    match serde_json::to_value(schemars::schema_for!(RawConfig)) {
        Ok(value) => value,
        Err(_) => serde_json::Value::Null,
    }
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
