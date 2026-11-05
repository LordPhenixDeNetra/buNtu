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
    CompiledConfig, CompiledKey, CompiledProvider, CompiledRoute, CompiledTarget, PROVIDER_KINDS,
    SecretValue,
};
pub use env::{Environment, MapEnvironment, ProcessEnv};
pub use file::{
    ConfigMode, RawConfig, RawKey, RawProvider, RawRoute, RawServer, RawTimeouts, json_schema,
};

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

/// Les deux couches persistantes de la configuration (cahier §7.1) ; l'environnement
/// s'applique au moment de la compilation.
#[derive(Debug, Clone, Default)]
pub struct Layers {
    /// Couche base : écrite par l'API d'administration, persistée en SQLite.
    pub base: RawConfig,
    /// Couche fichier : `buntu.toml`, rechargée à chaud.
    pub file: RawConfig,
}

/// Handle partagé vers la configuration compilée et ses couches.
///
/// `snapshot()` clone l'`Arc` : le `Guard` d'`ArcSwap` n'est jamais conservé à travers un
/// `.await`, ce qui évite d'épingler l'ancienne configuration.
#[derive(Debug, Clone)]
pub struct ConfigHandle {
    current: Arc<ArcSwap<CompiledConfig>>,
    layers: Arc<ArcSwap<Layers>>,
    /// Sérialise les écrivains (rechargement du fichier, API d'administration) : aucun ne
    /// publie une configuration compilée à partir d'une couche périmée.
    writer: Arc<std::sync::Mutex<()>>,
}

impl ConfigHandle {
    /// Configuration déjà compilée, sans couches (tests, démarrage sans fichier).
    pub fn new(config: CompiledConfig) -> Self {
        Self {
            current: Arc::new(ArcSwap::from_pointee(config)),
            layers: Arc::new(ArcSwap::from_pointee(Layers::default())),
            writer: Arc::new(std::sync::Mutex::new(())),
        }
    }

    /// Compile les couches base et fichier ; refuse une configuration invalide.
    pub fn from_layers(
        base: RawConfig,
        file: RawConfig,
        environment: &dyn Environment,
    ) -> Result<Self, ConfigError> {
        let compiled = compile_layers(&base, &file, environment)?;
        let handle = Self::new(compiled);
        handle.layers.store(Arc::new(Layers { base, file }));
        Ok(handle)
    }

    pub fn snapshot(&self) -> Arc<CompiledConfig> {
        self.current.load_full()
    }

    /// Publie une configuration déjà compilée sans toucher aux couches.
    pub fn store(&self, config: CompiledConfig) {
        self.current.store(Arc::new(config));
    }

    /// Couches actuellement publiées.
    pub fn layers(&self) -> Arc<Layers> {
        self.layers.load_full()
    }

    /// Recharge le fichier, le fusionne avec la base puis remplace atomiquement la
    /// configuration. En cas d'erreur, l'ancienne configuration reste active.
    pub fn reload(
        &self,
        path: &Path,
        environment: &dyn Environment,
    ) -> Result<Arc<CompiledConfig>, ConfigError> {
        let file = file::read(path)?;
        let _writer = self.lock_writer();
        let base = self.layers().base.clone();
        let compiled = Arc::new(compile_layers(&base, &file, environment)?);
        self.layers.store(Arc::new(Layers { base, file }));
        self.current.store(Arc::clone(&compiled));
        Ok(compiled)
    }

    /// Publie une nouvelle couche base (API d'administration) au-dessous du fichier actuel.
    pub fn commit_base(
        &self,
        base: RawConfig,
        environment: &dyn Environment,
    ) -> Result<Arc<CompiledConfig>, ConfigError> {
        let _writer = self.lock_writer();
        let file = self.layers().file.clone();
        let compiled = Arc::new(compile_layers(&base, &file, environment)?);
        self.layers.store(Arc::new(Layers { base, file }));
        self.current.store(Arc::clone(&compiled));
        Ok(compiled)
    }

    fn lock_writer(&self) -> std::sync::MutexGuard<'_, ()> {
        self.writer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Fusionne les couches : une entrée du fichier remplace celle de la base de même identifiant
/// (fournisseur) ou alias (route) ; les sections globales viennent du fichier seul.
///
/// Les entrées du fichier gardent leurs positions, pour que les index `BUNTU_PROVIDERS__0__…`
/// visent toujours le fichier.
pub fn merge(base: &RawConfig, file: &RawConfig) -> RawConfig {
    let mut merged = file.clone();
    for provider in &base.providers {
        if !file.providers.iter().any(|entry| entry.id == provider.id) {
            merged.providers.push(provider.clone());
        }
    }
    for route in &base.routes {
        if !file.routes.iter().any(|entry| entry.alias == route.alias) {
            merged.routes.push(route.clone());
        }
    }
    merged
}

/// Compile base < fichier < environnement (cahier §7.1).
pub fn compile_layers(
    base: &RawConfig,
    file: &RawConfig,
    environment: &dyn Environment,
) -> Result<CompiledConfig, ConfigError> {
    let raw = env::apply(merge(base, file), environment)?;
    compiled::compile(&raw, environment)
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
