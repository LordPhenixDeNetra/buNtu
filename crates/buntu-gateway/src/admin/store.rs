//! Couche « base » de la configuration (cahier §7.1, ADR-0008) : ce que l'API d'administration
//! écrit, persisté en SQLite, sous le fichier et l'environnement.
//!
//! Une mutation est validée par le **même code** que `buntu config validate` (fusion base <
//! fichier < environnement puis compilation), persistée, puis publiée. Un échec à n'importe
//! quelle étape ne publie rien.

use buntu_core::config::{
    ConfigMode, ProcessEnv, RawConfig, RawKey, RawProvider, RawRoute, compile_layers,
};
use buntu_schema::GatewayError;
use buntu_security::secrets::SecretCipher;
use buntu_store::{ProviderRecord, Store, StoredKey, StoredRoute, StoredSecret};

use crate::state::GatewayState;

/// Entrée de la couche base touchée par une mutation, à réécrire ou supprimer en SQLite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Touched {
    Provider(String),
    Route(String),
}

/// Écrivain unique de la couche base.
#[derive(Debug, Default)]
pub struct AdminStore {
    /// Sérialise les mutations (lecture, validation, écriture SQLite, publication).
    lock: tokio::sync::Mutex<()>,
    /// Chiffre les clés littérales au repos (T1.21) ; sans elle, seules les références
    /// `env:VAR` peuvent être persistées.
    cipher: Option<SecretCipher>,
}

impl AdminStore {
    pub fn new(cipher: Option<SecretCipher>) -> Self {
        Self {
            lock: tokio::sync::Mutex::new(()),
            cipher,
        }
    }

    /// Applique `change` à une copie de la couche base (le fichier est fourni en lecture),
    /// valide, persiste puis publie.
    pub async fn mutate<F>(&self, state: &GatewayState, change: F) -> Result<(), GatewayError>
    where
        F: FnOnce(&mut RawConfig, &RawConfig) -> Result<Touched, GatewayError>,
    {
        let _guard = self.lock.lock().await;
        if state.config.snapshot().mode == ConfigMode::File {
            return Err(invalid(
                "configuration en mode `file` : l'API d'administration est en lecture seule, \
                 modifiez le fichier de configuration",
            ));
        }
        let layers = state.config.layers();
        let mut candidate = layers.base.clone();
        let touched = change(&mut candidate, &layers.file)?;
        compile_layers(&candidate, &layers.file, &ProcessEnv)
            .map_err(|error| invalid(&error.to_string()))?;
        if let Some(store) = &state.store {
            self.persist(store, &candidate, &touched).await?;
        }
        state
            .config
            .commit_base(candidate, &ProcessEnv)
            .map_err(|error| invalid(&error.to_string()))?;
        Ok(())
    }

    async fn persist(
        &self,
        store: &Store,
        base: &RawConfig,
        touched: &Touched,
    ) -> Result<(), GatewayError> {
        match touched {
            Touched::Provider(id) => match base.providers.iter().find(|entry| &entry.id == id) {
                Some(provider) => {
                    let keys = provider
                        .keys
                        .iter()
                        .map(|key| self.stored_key(key))
                        .collect::<Result<Vec<_>, _>>()?;
                    let record = ProviderRecord {
                        id: provider.id.clone(),
                        kind: provider.kind.clone(),
                        base_url: provider.base_url.clone(),
                    };
                    store.save_provider(&record, &keys).await.map_err(storage)?;
                }
                None => {
                    store.delete_provider(id).await.map_err(storage)?;
                }
            },
            Touched::Route(alias) => match base.routes.iter().find(|entry| &entry.alias == alias) {
                Some(route) => {
                    let timeouts = serde_json::to_string(&route.timeouts)
                        .map_err(|error| internal(&error.to_string()))?;
                    store
                        .save_route(&StoredRoute {
                            alias: route.alias.clone(),
                            fallbacks: route.fallbacks.clone(),
                            timeouts,
                        })
                        .await
                        .map_err(storage)?;
                }
                None => {
                    store.delete_route(alias).await.map_err(storage)?;
                }
            },
        }
        Ok(())
    }

    fn stored_key(&self, key: &RawKey) -> Result<StoredKey, GatewayError> {
        let secret = if key.secret.starts_with("env:") {
            StoredSecret::Reference(key.secret.clone())
        } else {
            let cipher = self.cipher.as_ref().ok_or_else(|| {
                invalid(
                    "clé maître absente : définissez BUNTU_MASTER_KEY (ou son fichier) pour \
                     persister une clé, ou passez une référence `env:VAR`",
                )
            })?;
            StoredSecret::Encrypted(
                cipher
                    .encrypt(key.secret.as_bytes())
                    .map_err(|error| internal(&error.to_string()))?,
            )
        };
        Ok(StoredKey {
            secret,
            weight: key.weight,
        })
    }
}

/// Relit la couche base depuis SQLite (démarrage) ; les clés chiffrées exigent la clé maître.
pub async fn load_base(
    store: &Store,
    cipher: Option<&SecretCipher>,
) -> Result<RawConfig, GatewayError> {
    let mut base = RawConfig::default();
    for record in store.providers().await.map_err(storage)? {
        let mut keys = Vec::new();
        for key in store.stored_keys(&record.id).await.map_err(storage)? {
            let secret = match key.secret {
                StoredSecret::Reference(reference) => reference,
                StoredSecret::Encrypted(blob) => {
                    let cipher = cipher.ok_or_else(|| {
                        internal(&format!(
                            "clé maître requise pour déchiffrer les clés de `{}`",
                            record.id
                        ))
                    })?;
                    let plain = cipher
                        .decrypt(&blob)
                        .map_err(|error| internal(&error.to_string()))?;
                    String::from_utf8(plain).map_err(|_| internal("clé déchiffrée non UTF-8"))?
                }
            };
            keys.push(RawKey {
                secret,
                weight: key.weight,
            });
        }
        base.providers.push(RawProvider {
            id: record.id,
            kind: record.kind,
            base_url: record.base_url,
            keys,
        });
    }
    for route in store.stored_routes().await.map_err(storage)? {
        base.routes.push(RawRoute {
            alias: route.alias,
            fallbacks: route.fallbacks,
            timeouts: serde_json::from_str(&route.timeouts)
                .map_err(|error| internal(&error.to_string()))?,
        });
    }
    Ok(base)
}

fn invalid(message: &str) -> GatewayError {
    GatewayError::InvalidRequest {
        message: message.to_owned(),
    }
}

fn internal(message: &str) -> GatewayError {
    GatewayError::Internal {
        message: message.to_owned(),
    }
}

fn storage(error: buntu_store::StoreError) -> GatewayError {
    internal(&format!("stockage : {error}"))
}
