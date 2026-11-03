//! Chargement des fixtures de conformité.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;

/// Une fixture : statut, en-têtes et corps (objet JSON ou texte brut pour un flux).
#[derive(Debug, Deserialize)]
pub struct Fixture {
    pub status: u16,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    pub body: serde_json::Value,
}

impl Fixture {
    pub fn body_bytes(&self) -> Vec<u8> {
        match &self.body {
            serde_json::Value::String(text) => text.clone().into_bytes(),
            other => serde_json::to_vec(other).unwrap_or_default(),
        }
    }
}

/// Répertoire des fixtures d'un fournisseur, à la racine du dépôt.
pub fn directory(provider: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests")
        .join("fixtures")
        .join(provider)
}

/// Charge la fixture `<cas>.json` d'un fournisseur.
pub fn load(provider: &str, case: &str) -> Result<Fixture, String> {
    let path = directory(provider).join(format!("{case}.json"));
    let raw = std::fs::read_to_string(&path)
        .map_err(|error| format!("fixture {} illisible : {error}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("fixture {} invalide : {error}", path.display()))
}
