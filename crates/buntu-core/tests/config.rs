//! Tests de la configuration : fusion, validation, rechargement.

use std::error::Error;
use std::net::SocketAddr;
use std::path::PathBuf;

use buntu_core::config::{CompiledConfig, ConfigError, SecretValue, from_str, load};
use buntu_core::config::{ConfigHandle, MapEnvironment};

fn expect_error(result: Result<CompiledConfig, ConfigError>) -> ConfigError {
    match result {
        Ok(_) => panic!("une erreur de configuration était attendue"),
        Err(error) => error,
    }
}

const EXAMPLE: &str = r#"
[server]
listen = "127.0.0.1:8080"

[[providers]]
id = "openai"
type = "openai"
keys = [{ secret = "env:OPENAI_API_KEY", weight = 1 }]

[[providers]]
id = "anthropic"
type = "anthropic"
keys = [{ secret = "env:ANTHROPIC_API_KEY", weight = 3 }]

[[routes]]
alias = "chat-default"
fallbacks = ["openai/gpt-4o-mini", "anthropic/claude-haiku"]
"#;

fn environment() -> MapEnvironment {
    MapEnvironment::new([
        ("OPENAI_API_KEY", "sk-openai-test"),
        ("ANTHROPIC_API_KEY", "sk-anthropic-test"),
    ])
}

fn temp_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "buntu-config-test-{}-{name}.toml",
        std::process::id()
    ));
    path
}

#[test]
fn example_configuration_compiles() -> Result<(), Box<dyn Error>> {
    let config = from_str(EXAMPLE, &environment())?;
    assert_eq!(
        config.server.listen,
        "127.0.0.1:8080".parse::<SocketAddr>()?
    );
    assert_eq!(config.providers.len(), 2);
    assert_eq!(config.routes.len(), 1);

    let openai = config
        .providers
        .get("openai")
        .ok_or("fournisseur openai absent")?;
    assert_eq!(openai.kind, "openai");
    assert_eq!(openai.keys[0].weight, 1);
    assert_eq!(openai.keys[0].secret.expose(), "sk-openai-test");

    let route = config.routes.get("chat-default").ok_or("route absente")?;
    assert_eq!(route.fallbacks.len(), 2);
    assert_eq!(route.fallbacks[0].provider, "openai");
    assert_eq!(route.fallbacks[0].model, "gpt-4o-mini");
    Ok(())
}

#[test]
fn environment_overrides_file() -> Result<(), Box<dyn Error>> {
    let environment = MapEnvironment::new([
        ("OPENAI_API_KEY", "sk-openai-test"),
        ("ANTHROPIC_API_KEY", "sk-anthropic-test"),
        ("BUNTU_SERVER__LISTEN", "127.0.0.1:9999"),
        ("BUNTU_PROVIDERS__1__KEYS__0__WEIGHT", "5"),
    ]);
    let config = from_str(EXAMPLE, &environment)?;
    assert_eq!(
        config.server.listen,
        "127.0.0.1:9999".parse::<SocketAddr>()?
    );
    let anthropic = config
        .providers
        .get("anthropic")
        .ok_or("fournisseur anthropic absent")?;
    assert_eq!(anthropic.keys[0].weight, 5);
    Ok(())
}

#[test]
fn missing_environment_variable_is_reported_with_its_path() {
    let environment = MapEnvironment::new([("ANTHROPIC_API_KEY", "sk-anthropic-test")]);
    let error = expect_error(from_str(EXAMPLE, &environment));
    let message = error.to_string();
    assert!(message.contains("OPENAI_API_KEY"), "message : {message}");
    assert!(
        message.contains("providers[0].keys[0].secret"),
        "message : {message}"
    );
}

#[test]
fn invalid_listen_address_names_the_field() {
    let content = EXAMPLE.replace("127.0.0.1:8080", "pas-une-adresse");
    let error = expect_error(from_str(&content, &environment()));
    assert!(error.to_string().contains("server.listen"));
}

#[test]
fn unknown_provider_in_route_is_rejected() {
    let content = EXAMPLE.replace("anthropic/claude-haiku", "inconnu/modele");
    let error = expect_error(from_str(&content, &environment()));
    assert!(error.to_string().contains("routes[0].fallbacks[1]"));
}

#[test]
fn duplicate_provider_is_rejected() {
    let content = EXAMPLE.replace(r#"id = "anthropic""#, r#"id = "openai""#);
    let error = expect_error(from_str(&content, &environment()));
    assert!(error.to_string().contains("providers[1].id"));
}

#[test]
fn zero_weight_is_rejected() {
    let content = EXAMPLE.replace("weight = 3", "weight = 0");
    let error = expect_error(from_str(&content, &environment()));
    assert!(error.to_string().contains("weight"));
}

#[test]
fn secrets_are_redacted_in_debug_output() -> Result<(), Box<dyn Error>> {
    let config = from_str(EXAMPLE, &environment())?;
    let debug = format!("{config:?}");
    assert!(!debug.contains("sk-openai-test"), "secret fuité : {debug}");
    assert!(
        !debug.contains("sk-anthropic-test"),
        "secret fuité : {debug}"
    );
    assert!(SecretValue::new("secret".to_owned()).expose() == "secret");
    Ok(())
}

#[test]
fn invalid_reload_keeps_the_previous_configuration() -> Result<(), Box<dyn Error>> {
    let path = temp_path("reload-invalid");
    std::fs::write(&path, EXAMPLE)?;
    let environment = environment();
    let loaded = load(&path, &environment)?;
    let handle = ConfigHandle::new(loaded);
    let before = handle.snapshot().server.listen;

    std::fs::write(&path, "[server]\nlisten = \"invalide\"\n")?;
    let result = handle.reload(&path, &environment);
    assert!(result.is_err(), "le rechargement invalide doit échouer");
    assert_eq!(handle.snapshot().server.listen, before);

    let _ = std::fs::remove_file(&path);
    Ok(())
}

#[test]
fn snapshots_stay_consistent_during_reloads() -> Result<(), Box<dyn Error>> {
    let path = temp_path("reload-concurrent");
    std::fs::write(&path, EXAMPLE)?;
    let environment = MapEnvironment::new([
        ("OPENAI_API_KEY", "sk-openai-test"),
        ("ANTHROPIC_API_KEY", "sk-anthropic-test"),
    ]);
    let handle = ConfigHandle::new(load(&path, &environment)?);
    let expected: SocketAddr = "127.0.0.1:8080".parse()?;

    let reader = {
        let handle = handle.clone();
        std::thread::spawn(move || {
            for _ in 0..500 {
                let snapshot = handle.snapshot();
                if snapshot.server.listen != expected {
                    return Err("instantané incohérent".to_owned());
                }
                if snapshot.providers.len() != 2 {
                    return Err("fournisseurs incohérents".to_owned());
                }
            }
            Ok(())
        })
    };

    for _ in 0..50 {
        let config = load(&path, &environment)?;
        handle.store(config);
    }

    reader
        .join()
        .map_err(|_| "le lecteur a paniqué")?
        .map_err(|message| -> Box<dyn Error> { message.into() })?;

    let _ = std::fs::remove_file(&path);
    Ok(())
}

#[test]
fn default_listen_address_is_loopback_only() -> Result<(), Box<dyn std::error::Error>> {
    // Sans `[server]`, rien n'est exposé au réseau : l'exposition se demande explicitement.
    let config = from_str("", &MapEnvironment::default())?;
    assert_eq!(
        config.server.listen,
        "127.0.0.1:8080".parse::<SocketAddr>()?
    );
    assert_eq!(
        buntu_core::config::CompiledConfig::default().server.listen,
        "127.0.0.1:8080".parse::<SocketAddr>()?
    );
    Ok(())
}

// --- Trois sources, un seul modèle (cahier §7.1, revue 2026-11-05 section 2) ---------------

fn provider(id: &str, base_url: &str) -> buntu_core::config::RawProvider {
    buntu_core::config::RawProvider {
        id: id.to_owned(),
        kind: "openai".to_owned(),
        base_url: Some(base_url.to_owned()),
        keys: vec![buntu_core::config::RawKey {
            secret: "sk-test".to_owned(),
            weight: 1,
        }],
    }
}

fn base_layer() -> buntu_core::config::RawConfig {
    buntu_core::config::RawConfig {
        providers: vec![
            provider("db", "http://base"),
            provider("partage", "http://base"),
        ],
        ..Default::default()
    }
}

const FILE_LAYER: &str = r#"
[[providers]]
id = "partage"
type = "openai"
base_url = "http://fichier"
keys = [{ secret = "sk-fichier" }]
"#;

#[test]
fn file_layer_overrides_the_database_layer_by_id() -> Result<(), Box<dyn Error>> {
    let file = buntu_core::config::file::parse_str(FILE_LAYER, "test")?;
    let compiled =
        buntu_core::config::compile_layers(&base_layer(), &file, &MapEnvironment::default())?;
    assert_eq!(compiled.providers.len(), 2);
    assert_eq!(
        compiled.providers["partage"].base_url.as_deref(),
        Some("http://fichier"),
        "le fichier a priorité sur la base"
    );
    assert_eq!(
        compiled.providers["db"].base_url.as_deref(),
        Some("http://base")
    );
    Ok(())
}

#[test]
fn environment_overrides_the_file_and_the_database() -> Result<(), Box<dyn Error>> {
    let file = buntu_core::config::file::parse_str(FILE_LAYER, "test")?;
    let environment = MapEnvironment::new([("BUNTU_PROVIDERS__0__BASE_URL", "http://env")]);
    let compiled = buntu_core::config::compile_layers(&base_layer(), &file, &environment)?;
    assert_eq!(
        compiled.providers["partage"].base_url.as_deref(),
        Some("http://env")
    );
    Ok(())
}

#[test]
fn file_reload_keeps_the_database_layer() -> Result<(), Box<dyn Error>> {
    let path = temp_path("reload-layers");
    std::fs::write(&path, FILE_LAYER)?;
    let environment = MapEnvironment::default();
    let file = buntu_core::config::file::read(&path)?;
    let handle = ConfigHandle::from_layers(base_layer(), file, &environment)?;

    std::fs::write(
        &path,
        FILE_LAYER.replace("http://fichier", "http://fichier-v2"),
    )?;
    handle.reload(&path, &environment)?;
    let snapshot = handle.snapshot();
    assert!(
        snapshot.providers.contains_key("db"),
        "la couche base est perdue"
    );
    assert_eq!(
        snapshot.providers["partage"].base_url.as_deref(),
        Some("http://fichier-v2")
    );
    assert_eq!(handle.layers().base.providers.len(), 2);

    // Une modification de la base est publiée sans toucher au fichier.
    let mut base = handle.layers().base.clone();
    base.providers.retain(|provider| provider.id != "db");
    handle.commit_base(base, &environment)?;
    assert!(!handle.snapshot().providers.contains_key("db"));
    assert!(handle.snapshot().providers.contains_key("partage"));

    let _ = std::fs::remove_file(&path);
    Ok(())
}

#[test]
fn config_mode_file_makes_the_database_read_only() -> Result<(), Box<dyn Error>> {
    use buntu_core::config::ConfigMode;
    assert_eq!(
        from_str("", &MapEnvironment::default())?.mode,
        ConfigMode::Merged
    );
    let compiled = from_str("[config]\nmode = \"file\"\n", &MapEnvironment::default())?;
    assert_eq!(compiled.mode, ConfigMode::File);
    Ok(())
}

#[test]
fn unknown_provider_type_is_rejected_by_validation() {
    let content = EXAMPLE.replace("type = \"openai\"", "type = \"opnai\"");
    let error = expect_error(from_str(&content, &environment()));
    let message = error.to_string();
    assert!(message.contains("providers[0].type"), "{message}");
    assert!(message.contains("opnai"), "{message}");
}
