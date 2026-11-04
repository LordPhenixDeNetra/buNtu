//! Tests d'intégration de l'API d'administration (T1.16) et de l'OpenAPI.

use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use buntu_core::config::{ConfigHandle, RawConfig};
use buntu_gateway::admin::AdminStore;
use buntu_gateway::{GatewayState, Readiness, app_with_state};
use serde_json::{Value, json};

struct Running {
    address: SocketAddr,
    handle: tokio::task::JoinHandle<()>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

async fn start() -> Result<Running, Box<dyn Error>> {
    let state = GatewayState::new(
        Readiness::new(true),
        ConfigHandle::new(buntu_core::config::CompiledConfig::default()),
    )
    .with_admin(Arc::new(AdminStore::new(RawConfig::default())));
    let router: Router = app_with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    for _ in 0..100 {
        if reqwest::get(format!("http://{address}/health"))
            .await
            .map(|response| response.status().is_success())
            .unwrap_or(false)
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    Ok(Running { address, handle })
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

async fn create_provider(
    base: &str,
    id: &str,
    secret: &str,
) -> Result<reqwest::Response, Box<dyn Error>> {
    Ok(client()
        .post(format!("{base}/admin/v1/providers"))
        .json(&json!({
            "id": id,
            "type": "mock",
            "base_url": "http://127.0.0.1:9",
            "keys": [{ "secret": secret, "weight": 2 }],
        }))
        .send()
        .await?)
}

#[tokio::test]
async fn provider_crud_masks_secrets_and_paginates() -> Result<(), Box<dyn Error>> {
    let running = start().await?;
    let base = format!("http://{}", running.address);
    let response = create_provider(&base, "p1", "sk-supersecret-1234").await?;
    assert_eq!(response.status(), 201);
    let body: Value = response.json().await?;
    let preview = body["keys"][0]["preview"].as_str().unwrap_or_default();
    assert_eq!(preview, "sk-…1234");
    assert!(!preview.contains("supersecret"));
    assert_eq!(body["keys"][0]["weight"], 2);

    // Liste et pagination (3 fournisseurs, pages de 2).
    create_provider(&base, "p2", "sk-other-5678").await?;
    create_provider(&base, "p3", "sk-third-9012").await?;
    let page_one: Value = client()
        .get(format!("{base}/admin/v1/providers?limit=2"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(page_one["data"].as_array().map(Vec::len), Some(2));
    let cursor = page_one["next_cursor"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    assert!(!cursor.is_empty());
    let page_two: Value = client()
        .get(format!("{base}/admin/v1/providers?limit=2&cursor={cursor}"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(page_two["data"].as_array().map(Vec::len), Some(1));
    assert!(page_two["next_cursor"].is_null());

    // Aucune clé en clair dans la liste.
    let listing = client()
        .get(format!("{base}/admin/v1/providers"))
        .send()
        .await?
        .text()
        .await?;
    assert!(!listing.contains("supersecret"), "fuite : {listing}");
    Ok(())
}

#[tokio::test]
async fn provider_keys_and_routes_crud() -> Result<(), Box<dyn Error>> {
    let running = start().await?;
    let base = format!("http://{}", running.address);
    create_provider(&base, "p1", "sk-initial-0000").await?;

    let added: Value = client()
        .post(format!("{base}/admin/v1/providers/p1/keys"))
        .json(&json!({ "secret": "sk-added-1111", "weight": 1 }))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(added["preview"], "sk-…1111");

    let deleted = client()
        .delete(format!("{base}/admin/v1/providers/p1/keys/0"))
        .send()
        .await?;
    assert_eq!(deleted.status(), 204);
    let provider: Value = client()
        .get(format!("{base}/admin/v1/providers/p1"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(provider["keys"].as_array().map(Vec::len), Some(1));

    let created = client()
        .post(format!("{base}/admin/v1/routes"))
        .json(&json!({ "alias": "chat", "fallbacks": ["p1/modele-a"] }))
        .send()
        .await?;
    assert_eq!(created.status(), 201);

    let patched: Value = client()
        .patch(format!("{base}/admin/v1/routes/chat"))
        .json(&json!({ "fallbacks": ["p1/modele-b"] }))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(patched["fallbacks"][0], "p1/modele-b");

    let routes: Value = client()
        .get(format!("{base}/admin/v1/routes"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(routes["data"].as_array().map(Vec::len), Some(1));

    let removed = client()
        .delete(format!("{base}/admin/v1/routes/chat"))
        .send()
        .await?;
    assert_eq!(removed.status(), 204);
    let missing = client()
        .get(format!("{base}/admin/v1/routes/chat"))
        .send()
        .await?;
    assert_eq!(missing.status(), 400);
    Ok(())
}

#[tokio::test]
async fn invalid_configuration_is_rejected_by_the_same_validator() -> Result<(), Box<dyn Error>> {
    let running = start().await?;
    let base = format!("http://{}", running.address);
    let response = client()
        .post(format!("{base}/admin/v1/providers"))
        .json(&json!({
            "id": "p1",
            "type": "mock",
            "base_url": "http://127.0.0.1:9",
            "keys": [],
        }))
        .send()
        .await?;
    assert_eq!(response.status(), 400);
    let body: Value = response.json().await?;
    assert_eq!(body["error"]["code"], "invalid_request");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("clé"),
        "message : {body}"
    );
    Ok(())
}

#[tokio::test]
async fn openapi_endpoint_serves_the_specification() -> Result<(), Box<dyn Error>> {
    let running = start().await?;
    let base = format!("http://{}", running.address);
    let response = client()
        .get(format!("{base}/admin/v1/openapi.json"))
        .send()
        .await?;
    assert_eq!(response.status(), 200);
    let body: Value = response.json().await?;
    assert_eq!(body["info"]["title"], "buNtu Admin API");
    assert!(body["paths"]["/admin/v1/providers"].is_object());
    assert!(body["paths"]["/admin/v1/routes"].is_object());
    Ok(())
}
