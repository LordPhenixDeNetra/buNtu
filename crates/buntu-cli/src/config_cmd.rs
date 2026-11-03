//! Sous-commande `buntu config`.

use std::error::Error;
use std::path::Path;

use buntu_core::config::{CompiledConfig, ProcessEnv};

/// Valide un fichier de configuration et renvoie la configuration compilée.
pub fn validate(path: &Path) -> Result<CompiledConfig, Box<dyn Error>> {
    Ok(buntu_core::config::load(path, &ProcessEnv)?)
}
