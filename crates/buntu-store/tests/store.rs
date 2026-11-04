//! Preuves de T1.17 : base vide → migrations → écriture et relecture de chaque entité.

use std::error::Error;

use buntu_store::{ProviderRecord, RequestLogRecord, RouteRecord, Store};

#[tokio::test]
async fn migrations_apply_to_an_empty_database() -> Result<(), Box<dyn Error>> {
    let store = Store::open_in_memory().await?;
    // Deux migrations : 0001 initiale, 0002 colonne `attempts` (T1.24).
    assert_eq!(store.schema_version().await?, 2);
    Ok(())
}

#[tokio::test]
async fn every_entity_round_trips() -> Result<(), Box<dyn Error>> {
    let store = Store::open_in_memory().await?;
    let provider = ProviderRecord {
        id: "openai".to_owned(),
        kind: "openai".to_owned(),
        base_url: Some("http://127.0.0.1:9000".to_owned()),
    };
    store.insert_provider(&provider).await?;
    assert_eq!(store.provider("openai").await?, provider);
    assert_eq!(store.providers().await?.len(), 1);

    let key_id = store.insert_key("openai", b"ciphertext-only", 3).await?;
    let keys = store.keys("openai").await?;
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].id, key_id);
    assert_eq!(keys[0].weight, 3);
    assert_eq!(keys[0].secret_ciphertext, b"ciphertext-only");

    let fallbacks = vec!["openai/gpt-4o".to_owned(), "anthropic/claude".to_owned()];
    store.insert_route("chat", &fallbacks).await?;
    let route = store.route("chat").await?;
    assert_eq!(
        route,
        RouteRecord {
            alias: "chat".to_owned(),
            fallbacks,
        }
    );

    let log = RequestLogRecord {
        id: "req-1".to_owned(),
        model: "chat".to_owned(),
        provider: "openai".to_owned(),
        status: 200,
        latency_ms: 42,
        input_tokens: 10,
        output_tokens: 20,
        created_at: 1_700_000_000,
        attempts: 1,
        rowid: 1,
    };
    store.insert_request_log(&log).await?;
    let logs = store.recent_request_logs(10).await?;
    assert_eq!(logs, vec![log]);
    Ok(())
}

#[tokio::test]
async fn request_log_filters_and_cursor_paginate_server_side() -> Result<(), Box<dyn Error>> {
    let store = Store::open_in_memory().await?;
    for index in 0..3 {
        let log = RequestLogRecord {
            id: format!("req-{index}"),
            model: "chat".to_owned(),
            provider: if index == 2 {
                "anthropic".to_owned()
            } else {
                "mock".to_owned()
            },
            status: 200,
            latency_ms: 5,
            input_tokens: 1,
            output_tokens: 1,
            created_at: 1_700_000_000,
            attempts: 1,
            rowid: 0,
        };
        store.insert_request_log(&log).await?;
    }
    let page = store
        .query_request_logs(&buntu_store::RequestLogQuery {
            limit: 2,
            ..Default::default()
        })
        .await?;
    assert_eq!(page.len(), 2);
    assert!(
        page[0].rowid > page[1].rowid,
        "tri du plus récent au plus ancien"
    );
    let cursor = page[1].rowid;
    let next = store
        .query_request_logs(&buntu_store::RequestLogQuery {
            limit: 2,
            cursor: Some(cursor),
            ..Default::default()
        })
        .await?;
    assert_eq!(next.len(), 1);
    let filtered = store
        .query_request_logs(&buntu_store::RequestLogQuery {
            limit: 10,
            provider: Some("anthropic".to_owned()),
            ..Default::default()
        })
        .await?;
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].provider, "anthropic");
    let detail = store.request_log("req-0").await?;
    assert_eq!(detail.id, "req-0");
    assert!(store.request_log("absent").await.is_err());
    Ok(())
}
#[tokio::test]
async fn unknown_entities_are_reported() -> Result<(), Box<dyn Error>> {
    let store = Store::open_in_memory().await?;
    assert!(store.provider("absent").await.is_err());
    assert!(store.route("absente").await.is_err());
    Ok(())
}

#[tokio::test]
async fn revoked_api_token_is_inactive() -> Result<(), Box<dyn Error>> {
    let store = Store::open_in_memory().await?;
    store.insert_api_token("tok-1", "hash-1", "admin").await?;
    assert!(store.api_token_is_active("hash-1").await?);
    assert!(store.revoke_api_token("tok-1").await?);
    assert!(!store.api_token_is_active("hash-1").await?);
    assert!(!store.revoke_api_token("tok-absent").await?);
    assert!(store.api_token_is_active("hash-absent").await.is_err());
    Ok(())
}

#[tokio::test]
async fn keys_and_rows_survive_a_file_database_with_wal() -> Result<(), Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!("buntu-store-{}.sqlite", std::process::id()));
    let url = format!("sqlite://{}", path.display().to_string().replace('\\', "/"));
    let _ = std::fs::remove_file(&path);
    let store = Store::open(&url).await?;
    store
        .insert_provider(&ProviderRecord {
            id: "p".to_owned(),
            kind: "mock".to_owned(),
            base_url: None,
        })
        .await?;
    store.insert_key("p", b"blob", 1).await?;
    let journal: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(store.pool())
        .await?;
    assert_eq!(journal, "wal");
    drop(store);
    let reopened = Store::open(&url).await?;
    assert_eq!(reopened.keys("p").await?.len(), 1);
    drop(reopened);
    let _ = std::fs::remove_file(&path);
    Ok(())
}
