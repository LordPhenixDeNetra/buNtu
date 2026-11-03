//! Fusion avec l'environnement : variables `BUNTU_*` au-dessus du fichier.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::config::ConfigError;
use crate::config::file::RawConfig;

const PREFIX: &str = "BUNTU_";

/// Source d'environnement, injectable pour les tests.
pub trait Environment {
    fn get(&self, name: &str) -> Option<String>;
    fn variables(&self) -> Vec<(String, String)>;
}

/// Environnement réel du processus.
pub struct ProcessEnv;

impl Environment for ProcessEnv {
    fn get(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn variables(&self) -> Vec<(String, String)> {
        std::env::vars().collect()
    }
}

/// Environnement figé, utilisé par les tests.
#[derive(Debug, Default, Clone)]
pub struct MapEnvironment {
    values: BTreeMap<String, String>,
}

impl MapEnvironment {
    pub fn new<I, K, V>(values: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        Self {
            values: values
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        }
    }
}

impl Environment for MapEnvironment {
    fn get(&self, name: &str) -> Option<String> {
        self.values.get(name).cloned()
    }

    fn variables(&self) -> Vec<(String, String)> {
        self.values
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }
}

/// Applique les variables `BUNTU_*` au-dessus de la configuration issue du fichier.
///
/// Le chemin est encodé par `__` : `BUNTU_SERVER__LISTEN`, `BUNTU_PROVIDERS__0__ID`. Les
/// segments numériques indexent des tableaux déjà présents dans le fichier.
pub fn apply(raw: RawConfig, environment: &dyn Environment) -> Result<RawConfig, ConfigError> {
    let mut value = serde_json::to_value(&raw).map_err(|error| ConfigError::Invalid {
        path: "environment".to_owned(),
        message: error.to_string(),
    })?;

    for (name, raw_value) in environment.variables() {
        let Some(path) = name.strip_prefix(PREFIX) else {
            continue;
        };
        let segments: Vec<String> = path
            .split("__")
            .map(|segment| segment.to_lowercase())
            .collect();
        if segments.is_empty() || segments.iter().any(|segment| segment.is_empty()) {
            return Err(ConfigError::Invalid {
                path: format!("environment.{name}"),
                message: "chemin vide".to_owned(),
            });
        }
        set_path(&mut value, &segments, coerce(&raw_value)).map_err(|message| {
            ConfigError::Invalid {
                path: format!("environment.{name}"),
                message,
            }
        })?;
    }

    serde_json::from_value(value).map_err(|error| ConfigError::Invalid {
        path: "environment".to_owned(),
        message: error.to_string(),
    })
}

fn coerce(raw: &str) -> Value {
    if raw.eq_ignore_ascii_case("true") {
        return Value::Bool(true);
    }
    if raw.eq_ignore_ascii_case("false") {
        return Value::Bool(false);
    }
    if let Ok(number) = raw.parse::<i64>() {
        return Value::from(number);
    }
    if let Ok(number) = raw.parse::<u64>() {
        return Value::from(number);
    }
    Value::String(raw.to_owned())
}

fn set_path(target: &mut Value, segments: &[String], new_value: Value) -> Result<(), String> {
    let Some((head, rest)) = segments.split_first() else {
        *target = new_value;
        return Ok(());
    };

    match target {
        Value::Object(map) => {
            if rest.is_empty() {
                map.insert(head.clone(), new_value);
                return Ok(());
            }
            if !map.contains_key(head) {
                if is_index(&rest[0]) {
                    return Err(format!("`{head}` devrait être un tableau"));
                }
                map.insert(head.clone(), Value::Object(Map::new()));
            }
            let entry = map
                .get_mut(head)
                .ok_or_else(|| format!("`{head}` introuvable"))?;
            set_path(entry, rest, new_value)
        }
        Value::Array(items) => {
            let index = parse_index(head)?;
            let slot = items
                .get_mut(index)
                .ok_or_else(|| format!("index {index} hors limites"))?;
            set_path(slot, rest, new_value)
        }
        _ => Err(format!(
            "`{head}` ne peut pas être placé dans une valeur scalaire"
        )),
    }
}

fn is_index(segment: &str) -> bool {
    !segment.is_empty() && segment.bytes().all(|byte| byte.is_ascii_digit())
}

fn parse_index(segment: &str) -> Result<usize, String> {
    if !is_index(segment) {
        return Err(format!("`{segment}` n'est pas un index de tableau"));
    }
    segment
        .parse()
        .map_err(|_| format!("index invalide : {segment}"))
}
