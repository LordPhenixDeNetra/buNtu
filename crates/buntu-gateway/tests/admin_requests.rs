//! Preuves T1.24 : journal paginé/filtré et détail via l'API d'administration.

use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use buntu_core::config::{ConfigHandle, RawConfig};
use buntu_gateway::admin::AdminStore;
use buntu_gateway::{GatewayState, Readiness, app_with_state};
use buntu_store::{RequestLogRecord, Store};
use serde_json::Value;

/// Jeton admin des tests (API protégée, revue 2026-11-05 point 1a).
const ADMIN_TOKEN: &str = "buntu_jeton-de-test-admin-0123456789";

async fn admin_get(url: String) -> reqwest::Result<reqwest::Response> {
    reqwest::Client::new()
        .get(url)
        .bearer_auth(ADMIN_TOKEN)
        .send()
        .await
}

async fn start() -> Result<(SocketAddr, Store, tokio::task::JoinHandle<()>), Box<dyn Error>> {
    let store = Store::open_in_memory().await?;
    let state = GatewayState::new(
        Readiness::new(true),
        ConfigHandle::new(buntu_core::config::CompiledConfig::default()),
    )
    .with_admin(Arc::new(AdminStore::new(RawConfig::default())))
    .with_storage(Arc::new(store.clone()))
    .with_admin_token(ADMIN_TOKEN);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app_with_state(state)).await;
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
    Ok((address, store, handle))
}

async fn insert(store: &Store, index: usize, provider: &str) -> Result<(), Box<dyn Error>> {
    store
        .insert_request_log(&RequestLogRecord {
            id: format!("req-{index}"),
            model: "chat".to_owned(),
            provider: provider.to_owned(),
            status: 200,
            latency_ms: 12,
            input_tokens: 2,
            output_tokens: 3,
            created_at: 1_700_000_000 + index as i64,
            attempts: 1,
            rowid: 0,
        })
        .await?;
    Ok(())
}

#[tokio::test]
async fn journal_is_paginated_filtered_and_has_a_detail() -> Result<(), Box<dyn Error>> {
    let (address, store, handle) = start().await?;
    insert(&store, 0, "mock").await?;
    insert(&store, 1, "mock").await?;
    insert(&store, 2, "anthropic").await?;

    let page: Value = admin_get(format!("http://{address}/admin/v1/requests?limit=2"))
        .await?
        .json()
        .await?;
    assert_eq!(page["data"].as_array().map(Vec::len), Some(2));
    assert!(page["next_cursor"].as_i64().is_some());
    assert_eq!(page["data"][0]["attempts"], 1);
    assert_eq!(page["data"][0]["model"], "chat");

    let cursor = page["next_cursor"].as_i64().unwrap_or_default();
    let rest: Value = admin_get(format!(
        "http://{address}/admin/v1/requests?limit=2&cursor={cursor}"
    ))
    .await?
    .json()
    .await?;
    assert_eq!(rest["data"].as_array().map(Vec::len), Some(1));
    assert!(rest["next_cursor"].is_null());

    let filtered: Value = admin_get(format!(
        "http://{address}/admin/v1/requests?provider=anthropic"
    ))
    .await?
    .json()
    .await?;
    assert_eq!(filtered["data"].as_array().map(Vec::len), Some(1));
    assert_eq!(filtered["data"][0]["provider"], "anthropic");

    let detail: Value = admin_get(format!("http://{address}/admin/v1/requests/req-0"))
        .await?
        .json()
        .await?;
    assert_eq!(detail["id"], "req-0");
    assert_eq!(detail["latency_ms"], 12);
    assert_eq!(detail["attempts"], 1);

    let missing = admin_get(format!("http://{address}/admin/v1/requests/absent")).await?;
    assert_eq!(missing.status(), 400);
    handle.abort();
    Ok(())
}
