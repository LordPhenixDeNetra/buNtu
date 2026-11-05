//! Une seule source de vérité pour la configuration (cahier §7.1, revue 2026-11-05 section 2).
//!
//! Base (API d'administration) < fichier < environnement ; le rechargement du fichier ne perd
//! jamais les changements faits par l'API, qui sont persistés en SQLite.

use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;

use buntu_core::config::{ConfigHandle, MapEnvironment, RawConfig, file::parse_str};
use buntu_gateway::admin::{AdminStore, load_base};
use buntu_gateway::{GatewayState, Readiness, app_with_state};
use buntu_security::secrets::SecretCipher;
use buntu_store::{Store, StoredSecret};
use serde_json::{Value, json};

const TOKEN: &str = "buntu_jeton-de-test-config-0123456789";

struct Running {
    address: SocketAddr,
    handle: tokio::task::JoinHandle<()>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

async fn start(state: GatewayState) -> Result<Running, Box<dyn Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let router = app_with_state(state.with_admin_token(TOKEN));
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    Ok(Running { address, handle })
}

fn handle(file: &str) -> Result<ConfigHandle, Box<dyn Error>> {
    Ok(ConfigHandle::from_layers(
        RawConfig::default(),
        parse_str(file, "test")?,
        &MapEnvironment::default(),
    )?)
}

async fn send(
    method: reqwest::Method,
    url: String,
    body: Option<Value>,
) -> Result<(u16, Value), Box<dyn Error>> {
    let mut request = reqwest::Client::new()
        .request(method, url)
        .bearer_auth(TOKEN);
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request.send().await?;
    let status = response.status().as_u16();
    let text = response.text().await?;
    Ok((status, serde_json::from_str(&text).unwrap_or(Value::Null)))
}

fn provider(id: &str, secret: &str) -> Value {
    json!({
        "id": id,
        "type": "mock",
        "base_url": "http://127.0.0.1:9",
        "keys": [{ "secret": secret, "weight": 1 }],
    })
}

#[tokio::test]
async fn admin_changes_are_persisted_and_secrets_encrypted() -> Result<(), Box<dyn Error>> {
    let store = Store::open_in_memory().await?;
    let cipher = SecretCipher::from_master_key(&[7_u8; 32])?;
    let state = GatewayState::new(Readiness::new(true), handle("")?)
        .with_admin(Arc::new(AdminStore::new(Some(cipher.clone()))))
        .with_storage(Arc::new(store.clone()));
    let running = start(state).await?;
    let base = format!("http://{}/admin/v1", running.address);

    let (status, _) = send(
        reqwest::Method::POST,
        format!("{base}/providers"),
        Some(provider("db", "sk-litteral-secret-1234")),
    )
    .await?;
    assert_eq!(status, 201);
    let (status, _) = send(
        reqwest::Method::POST,
        format!("{base}/routes"),
        Some(json!({ "alias": "chat-db", "fallbacks": ["db/modele"] })),
    )
    .await?;
    assert_eq!(status, 201);

    // La clé littérale est chiffrée au repos.
    let keys = store.stored_keys("db").await?;
    let StoredSecret::Encrypted(blob) = &keys[0].secret else {
        panic!("clé littérale attendue chiffrée : {keys:?}");
    };
    assert!(
        !blob
            .windows(b"sk-litteral".len())
            .any(|window| window == b"sk-litteral")
    );

    // « Redémarrage » : la couche base relue depuis SQLite contient les changements.
    let reloaded = load_base(&store, Some(&cipher)).await?;
    assert_eq!(reloaded.providers.len(), 1);
    assert_eq!(
        reloaded.providers[0].keys[0].secret,
        "sk-litteral-secret-1234"
    );
    assert_eq!(reloaded.routes[0].alias, "chat-db");

    let (status, _) = send(
        reqwest::Method::DELETE,
        format!("{base}/routes/chat-db"),
        None,
    )
    .await?;
    assert_eq!(status, 204);
    assert!(load_base(&store, Some(&cipher)).await?.routes.is_empty());
    Ok(())
}

#[tokio::test]
async fn literal_keys_need_a_master_key_when_persisted() -> Result<(), Box<dyn Error>> {
    let store = Store::open_in_memory().await?;
    let state = GatewayState::new(Readiness::new(true), handle("")?)
        .with_admin(Arc::new(AdminStore::new(None)))
        .with_storage(Arc::new(store.clone()));
    let running = start(state).await?;
    let base = format!("http://{}/admin/v1", running.address);

    let (status, body) = send(
        reqwest::Method::POST,
        format!("{base}/providers"),
        Some(provider("db", "sk-litteral")),
    )
    .await?;
    assert_eq!(status, 400);
    assert!(body.to_string().contains("clé maître"), "{body}");
    assert!(
        store.providers().await?.is_empty(),
        "rien n'est publié ni écrit"
    );

    // Une référence `env:` n'est pas un secret : elle est acceptée sans clé maître.
    // (`PATH` existe partout : la validation commune vérifie que la variable est définie.)
    let (status, _) = send(
        reqwest::Method::POST,
        format!("{base}/providers"),
        Some(provider("db", "env:PATH")),
    )
    .await?;
    assert_eq!(status, 201);
    let keys = store.stored_keys("db").await?;
    assert_eq!(
        keys[0].secret,
        StoredSecret::Reference("env:PATH".to_owned())
    );
    Ok(())
}

#[tokio::test]
async fn file_entries_win_and_survive_reloads_with_admin_changes() -> Result<(), Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!("buntu-layers-{}.toml", std::process::id()));
    let file = r#"
[[providers]]
id = "fichier"
type = "mock"
base_url = "http://127.0.0.1:9"
keys = [{ secret = "sk-fichier" }]
"#;
    std::fs::write(&path, file)?;
    let config = handle(file)?;
    let state = GatewayState::new(Readiness::new(true), config.clone())
        .with_admin(Arc::new(AdminStore::default()));
    let running = start(state).await?;
    let base = format!("http://{}/admin/v1", running.address);

    let (status, _) = send(
        reqwest::Method::POST,
        format!("{base}/providers"),
        Some(provider("db", "sk-db")),
    )
    .await?;
    assert_eq!(status, 201);

    // Une entrée du fichier est prioritaire : l'API refuse de la modifier.
    let (status, body) = send(
        reqwest::Method::PATCH,
        format!("{base}/providers/fichier"),
        Some(json!({ "base_url": "http://ailleurs" })),
    )
    .await?;
    assert_eq!(status, 400);
    assert!(
        body.to_string().contains("fichier de configuration"),
        "{body}"
    );
    let (status, _) = send(
        reqwest::Method::POST,
        format!("{base}/providers"),
        Some(provider("fichier", "sk-x")),
    )
    .await?;
    assert_eq!(status, 400);

    // Rechargement du fichier : le fournisseur créé par l'API reste là.
    std::fs::write(&path, file.replace("sk-fichier", "sk-fichier-v2"))?;
    config.reload(&path, &MapEnvironment::default())?;
    let (_, listing) = send(reqwest::Method::GET, format!("{base}/providers"), None).await?;
    let sources: Vec<(String, String)> = listing["data"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    (
                        item["id"].as_str().unwrap_or_default().to_owned(),
                        item["source"].as_str().unwrap_or_default().to_owned(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(
        sources,
        vec![
            ("db".to_owned(), "database".to_owned()),
            ("fichier".to_owned(), "file".to_owned()),
        ]
    );
    assert!(config.snapshot().providers.contains_key("db"));
    let _ = std::fs::remove_file(&path);
    Ok(())
}

#[tokio::test]
async fn file_mode_makes_the_admin_api_read_only() -> Result<(), Box<dyn Error>> {
    let state = GatewayState::new(Readiness::new(true), handle("[config]\nmode = \"file\"\n")?)
        .with_admin(Arc::new(AdminStore::default()));
    let running = start(state).await?;
    let base = format!("http://{}/admin/v1", running.address);
    let (status, body) = send(
        reqwest::Method::POST,
        format!("{base}/providers"),
        Some(provider("db", "sk-db")),
    )
    .await?;
    assert_eq!(status, 400);
    assert!(body.to_string().contains("mode"), "{body}");
    let (status, _) = send(reqwest::Method::GET, format!("{base}/providers"), None).await?;
    assert_eq!(status, 200, "la lecture reste permise");
    Ok(())
}

#[test]
fn registry_and_validation_know_the_same_provider_types() {
    let registry = buntu_gateway::registry::ProviderRegistry::new();
    let mut known: Vec<&str> = buntu_core::config::PROVIDER_KINDS.to_vec();
    known.sort_unstable();
    assert_eq!(registry.kinds(), known);
}
