//! Configuration : chargement, fusion, compilation et rechargement à chaud.
//!
//! Deux niveaux de types : `RawConfig` (le TOML, tolérant) et `CompiledConfig` (validée,
//! indexée, prête pour le chemin chaud). Le rechargement remplace atomiquement la
//! configuration compilée ; une configuration invalide est rejetée et l'ancienne reste active.

pub mod compiled;
pub mod env;
pub mod file;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;
use notify::Watcher;

pub use compiled::{
    CompiledConfig, CompiledKey, CompiledProvider, CompiledRoute, CompiledTarget, SecretValue,
};
pub use env::{Environment, MapEnvironment, ProcessEnv};
pub use file::{RawConfig, RawKey, RawProvider, RawRoute, RawServer, json_schema};

/// Erreurs de configuration, toujours accompagnées du chemin du champ fautif.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("lecture impossible de `{path}` : {source}")]
    Read {
        path: String,
        source: std::io::Error,
    },
    #[error("TOML invalide dans `{path}` : {source}")]
    Toml {
        path: String,
        source: toml::de::Error,
    },
    #[error("configuration invalide en `{path}` : {message}")]
    Invalid { path: String, message: String },
    #[error("variable d'environnement `{variable}` absente pour `{path}`")]
    MissingEnv { path: String, variable: String },
}

/// Handle partagé vers la configuration compilée.
///
/// `snapshot()` clone l'`Arc` : le `Guard` d'`ArcSwap` n'est jamais conservé à travers un
/// `.await`, ce qui évite d'épingler l'ancienne configuration.
#[derive(Debug, Clone)]
pub struct ConfigHandle {
    current: Arc<ArcSwap<CompiledConfig>>,
}

impl ConfigHandle {
    pub fn new(config: CompiledConfig) -> Self {
        Self {
            current: Arc::new(ArcSwap::from_pointee(config)),
        }
    }

    pub fn snapshot(&self) -> Arc<CompiledConfig> {
        self.current.load_full()
    }

    pub fn store(&self, config: CompiledConfig) {
        self.current.store(Arc::new(config));
    }

    /// Recharge le fichier puis remplace atomiquement la configuration.
    ///
    /// En cas d'erreur, l'ancienne configuration reste active et l'erreur est renvoyée.
    pub fn reload(
        &self,
        path: &Path,
        environment: &dyn Environment,
    ) -> Result<Arc<CompiledConfig>, ConfigError> {
        let config = load(path, environment)?;
        let config = Arc::new(config);
        self.current.store(Arc::clone(&config));
        Ok(config)
    }
}

/// Charge, fusionne avec l'environnement et compile un fichier de configuration.
pub fn load(path: &Path, environment: &dyn Environment) -> Result<CompiledConfig, ConfigError> {
    let raw = file::read(path)?;
    let raw = env::apply(raw, environment)?;
    compiled::compile(&raw, environment)
}

/// Parse et compile une configuration déjà en mémoire.
pub fn from_str(
    content: &str,
    environment: &dyn Environment,
) -> Result<CompiledConfig, ConfigError> {
    let raw = file::parse_str(content, "<mémoire>")?;
    let raw = env::apply(raw, environment)?;
    compiled::compile(&raw, environment)
}

/// Surveille le fichier et recharge la configuration à chaque modification, avec anti-rebond.
///
/// Le thread retourné vit aussi longtemps que le processus ; les erreurs de rechargement sont
/// journalisées et l'ancienne configuration est conservée.
pub fn spawn_file_watcher(
    handle: ConfigHandle,
    path: PathBuf,
    debounce: Duration,
) -> Result<std::thread::JoinHandle<()>, ConfigError> {
    if !path.exists() {
        return Err(ConfigError::Read {
            path: path.display().to_string(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "fichier absent"),
        });
    }

    let (sender, receiver) = std::sync::mpsc::channel::<()>();
    let watch_path = path.clone();

    Ok(std::thread::spawn(move || {
        let mut watcher =
            match notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                if event.is_ok() {
                    let _ = sender.send(());
                }
            }) {
                Ok(watcher) => watcher,
                Err(error) => {
                    eprintln!("buNtu: surveillance de la configuration impossible : {error}");
                    return;
                }
            };
        if let Err(error) = watcher.watch(&watch_path, notify::RecursiveMode::NonRecursive) {
            eprintln!("buNtu: surveillance de la configuration impossible : {error}");
            return;
        }

        let environment = ProcessEnv;
        loop {
            if receiver.recv().is_err() {
                return;
            }
            loop {
                match receiver.recv_timeout(debounce) {
                    Ok(()) => continue,
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
            if let Err(error) = handle.reload(&path, &environment) {
                eprintln!(
                    "buNtu: rechargement refusé ({error}) ; ancienne configuration conservée"
                );
            }
        }
    }))
}
