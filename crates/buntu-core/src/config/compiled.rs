//! Niveau compilé : validé, indexé, prêt pour le chemin chaud.

use std::collections::HashMap;
use std::fmt;
use std::net::SocketAddr;

use crate::config::file::{RawConfig, RawHooks, RawProvider, RawRoute};
use crate::config::{ConfigError, Environment};

#[derive(Debug, Clone)]
pub struct CompiledConfig {
    pub server: CompiledServer,
    pub providers: HashMap<String, CompiledProvider>,
    pub routes: HashMap<String, CompiledRoute>,
    pub hooks: CompiledHooks,
}

impl Default for CompiledConfig {
    fn default() -> Self {
        Self {
            server: CompiledServer {
                listen: SocketAddr::from(([0, 0, 0, 0], 8080)),
            },
            providers: HashMap::new(),
            routes: HashMap::new(),
            hooks: CompiledHooks::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompiledServer {
    pub listen: SocketAddr,
}

/// Chaîne de hooks déclarée par la configuration, dans l'ordre d'exécution.
#[derive(Debug, Clone, Default)]
pub struct CompiledHooks {
    pub pre: Vec<String>,
    pub post: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CompiledProvider {
    pub id: String,
    pub kind: String,
    pub keys: Vec<CompiledKey>,
}

#[derive(Debug, Clone)]
pub struct CompiledKey {
    pub secret: SecretValue,
    pub weight: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledTarget {
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Clone)]
pub struct CompiledRoute {
    pub alias: String,
    pub fallbacks: Vec<CompiledTarget>,
}

/// Valeur secrète : `Debug` affiche `***`, jamais la valeur.
///
/// Le type `Secret<T>` complet du cahier arrive avec T1.21 ; celui-ci est le minimum pour ne
/// pas laisser fuir une clé dans un journal.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretValue(String);

impl SecretValue {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("***")
    }
}

pub fn compile(
    raw: &RawConfig,
    environment: &dyn Environment,
) -> Result<CompiledConfig, ConfigError> {
    let listen: SocketAddr = raw
        .server
        .listen
        .parse()
        .map_err(|error| ConfigError::Invalid {
            path: "server.listen".to_owned(),
            message: format!("adresse invalide : {error}"),
        })?;

    let mut providers: HashMap<String, CompiledProvider> = HashMap::new();
    for (index, provider) in raw.providers.iter().enumerate() {
        let compiled = compile_provider(provider, index, environment)?;
        if providers.insert(compiled.id.clone(), compiled).is_some() {
            return Err(ConfigError::Invalid {
                path: format!("providers[{index}].id"),
                message: "identifiant de fournisseur en double".to_owned(),
            });
        }
    }

    let mut routes: HashMap<String, CompiledRoute> = HashMap::new();
    for (index, route) in raw.routes.iter().enumerate() {
        let compiled = compile_route(route, index, &providers)?;
        if routes.insert(compiled.alias.clone(), compiled).is_some() {
            return Err(ConfigError::Invalid {
                path: format!("routes[{index}].alias"),
                message: "alias de route en double".to_owned(),
            });
        }
    }

    let hooks = compile_hooks(&raw.hooks)?;

    Ok(CompiledConfig {
        server: CompiledServer { listen },
        providers,
        routes,
        hooks,
    })
}

fn compile_provider(
    provider: &RawProvider,
    index: usize,
    environment: &dyn Environment,
) -> Result<CompiledProvider, ConfigError> {
    let id = provider.id.trim();
    if id.is_empty() {
        return Err(ConfigError::Invalid {
            path: format!("providers[{index}].id"),
            message: "identifiant vide".to_owned(),
        });
    }
    if provider.kind.trim().is_empty() {
        return Err(ConfigError::Invalid {
            path: format!("providers[{index}].type"),
            message: "type vide".to_owned(),
        });
    }
    if provider.keys.is_empty() {
        return Err(ConfigError::Invalid {
            path: format!("providers[{index}].keys"),
            message: "au moins une clé est requise".to_owned(),
        });
    }

    let mut keys = Vec::with_capacity(provider.keys.len());
    for (key_index, key) in provider.keys.iter().enumerate() {
        if key.weight == 0 {
            return Err(ConfigError::Invalid {
                path: format!("providers[{index}].keys[{key_index}].weight"),
                message: "le poids doit être supérieur à zéro".to_owned(),
            });
        }
        let path = format!("providers[{index}].keys[{key_index}].secret");
        let secret = resolve_secret(&key.secret, &path, environment)?;
        keys.push(CompiledKey {
            secret,
            weight: key.weight,
        });
    }

    Ok(CompiledProvider {
        id: id.to_owned(),
        kind: provider.kind.trim().to_owned(),
        keys,
    })
}

fn resolve_secret(
    reference: &str,
    path: &str,
    environment: &dyn Environment,
) -> Result<SecretValue, ConfigError> {
    if let Some(variable) = reference.strip_prefix("env:") {
        let value = environment
            .get(variable)
            .ok_or_else(|| ConfigError::MissingEnv {
                path: path.to_owned(),
                variable: variable.to_owned(),
            })?;
        return Ok(SecretValue::new(value));
    }
    if reference.starts_with("secret:") {
        return Err(ConfigError::Invalid {
            path: path.to_owned(),
            message: "les références `secret:` arrivent avec T1.21".to_owned(),
        });
    }
    Ok(SecretValue::new(reference.to_owned()))
}

fn compile_route(
    route: &RawRoute,
    index: usize,
    providers: &HashMap<String, CompiledProvider>,
) -> Result<CompiledRoute, ConfigError> {
    let alias = route.alias.trim();
    if alias.is_empty() {
        return Err(ConfigError::Invalid {
            path: format!("routes[{index}].alias"),
            message: "alias vide".to_owned(),
        });
    }
    if route.fallbacks.is_empty() {
        return Err(ConfigError::Invalid {
            path: format!("routes[{index}].fallbacks"),
            message: "au moins une cible de repli est requise".to_owned(),
        });
    }

    let mut fallbacks = Vec::with_capacity(route.fallbacks.len());
    for (target_index, target) in route.fallbacks.iter().enumerate() {
        let path = format!("routes[{index}].fallbacks[{target_index}]");
        fallbacks.push(parse_target(target, &path, providers)?);
    }

    Ok(CompiledRoute {
        alias: alias.to_owned(),
        fallbacks,
    })
}

fn parse_target(
    target: &str,
    path: &str,
    providers: &HashMap<String, CompiledProvider>,
) -> Result<CompiledTarget, ConfigError> {
    let (provider, model) = target.split_once('/').ok_or_else(|| ConfigError::Invalid {
        path: path.to_owned(),
        message: "attendu `fournisseur/modèle`".to_owned(),
    })?;
    if provider.trim().is_empty() || model.trim().is_empty() {
        return Err(ConfigError::Invalid {
            path: path.to_owned(),
            message: "fournisseur ou modèle vide".to_owned(),
        });
    }
    if !providers.contains_key(provider) {
        return Err(ConfigError::Invalid {
            path: path.to_owned(),
            message: format!("fournisseur inconnu `{provider}`"),
        });
    }
    Ok(CompiledTarget {
        provider: provider.to_owned(),
        model: model.to_owned(),
    })
}

fn compile_hooks(hooks: &RawHooks) -> Result<CompiledHooks, ConfigError> {
    check_hook_names(&hooks.pre, "hooks.pre")?;
    check_hook_names(&hooks.post, "hooks.post")?;
    Ok(CompiledHooks {
        pre: hooks.pre.clone(),
        post: hooks.post.clone(),
    })
}

fn check_hook_names(names: &[String], section: &str) -> Result<(), ConfigError> {
    let mut seen = std::collections::HashSet::new();
    for (index, name) in names.iter().enumerate() {
        if name.trim().is_empty() {
            return Err(ConfigError::Invalid {
                path: format!("{section}[{index}]"),
                message: "nom de hook vide".to_owned(),
            });
        }
        if !seen.insert(name.as_str()) {
            return Err(ConfigError::Invalid {
                path: format!("{section}[{index}]"),
                message: "hook en double".to_owned(),
            });
        }
    }
    Ok(())
}
