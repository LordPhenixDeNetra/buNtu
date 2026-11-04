//! Magasin d'administration : configuration brute mutable, validée par le **même code** que
//! `buntu config validate` (`buntu_core::config::from_str`) avant d'être publiée.

use std::sync::RwLock;

use buntu_core::config::{CompiledConfig, RawConfig, from_str};
use buntu_schema::GatewayError;

/// Configuration brute partagée avec l'API d'administration.
pub struct AdminStore {
    raw: RwLock<RawConfig>,
}

impl std::fmt::Debug for AdminStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("AdminStore")
    }
}

impl Default for AdminStore {
    fn default() -> Self {
        Self::empty()
    }
}

impl AdminStore {
    pub fn new(raw: RawConfig) -> Self {
        Self {
            raw: RwLock::new(raw),
        }
    }

    pub fn empty() -> Self {
        Self::new(RawConfig::default())
    }

    /// Copie de la configuration brute (jamais modifiée en place par l'appelant).
    pub fn snapshot(&self) -> RawConfig {
        self.raw
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    /// Applique une mutation, valide le résultat, puis publie la configuration compilée.
    pub fn mutate<F>(&self, change: F) -> Result<CompiledConfig, GatewayError>
    where
        F: FnOnce(&mut RawConfig) -> Result<(), GatewayError>,
    {
        let mut guard = self
            .raw
            .write()
            .map_err(|_| internal("verrou d'administration"))?;
        let mut candidate = guard.clone();
        change(&mut candidate)?;
        // Validation identique à `config validate` (même fonction, même environnement).
        let toml = toml::to_string(&candidate).map_err(|error| GatewayError::InvalidRequest {
            message: format!("configuration non sérialisable : {error}"),
        })?;
        let compiled = from_str(&toml, &buntu_core::config::ProcessEnv).map_err(|error| {
            GatewayError::InvalidRequest {
                message: error.to_string(),
            }
        })?;
        *guard = candidate;
        Ok(compiled)
    }
}

fn internal(message: &str) -> GatewayError {
    GatewayError::Internal {
        message: message.to_owned(),
    }
}
