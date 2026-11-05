//! Toutes les routes alimentent le journal, succès comme erreurs (revue 2026-11-05, section 2).

use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use buntu_core::config::{ConfigHandle, MapEnvironment, from_str};
use buntu_gateway::{GatewayState, Readiness, app_with_state};
use buntu_store::Store;
use serde_json::json;

struct Running {
    address: SocketAddr,
    handle: tokio::task::JoinHandle<()>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

async fn spawn(router: Router) -> Result<Running, Box<dyn Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    Ok(Running { address, handle })
}

#[tokio::test]
async fn every_route_and_every_outcome_is_journaled() -> Result<(), Box<dyn Error>> {
    let mock = spawn(buntu_mock_provider::app()).await?;
    let config = from_str(
        &format!(
            r#"
[[providers]]
id = "mock"
type = "mock"
base_url = "http://{}"
keys = [{{ secret = "sk-test" }}]

[[routes]]
alias = "chat"
fallbacks = ["mock/fake-model"]
"#,
            mock.address
        ),
        &MapEnvironment::default(),
    )?;
    let store = Store::open_in_memory().await?;
    let state = GatewayState::new(Readiness::new(true), ConfigHandle::new(config))
        .with_storage(Arc::new(store.clone()));
    let gateway = spawn(app_with_state(state)).await?;
    let base = format!("http://{}", gateway.address);
    let client = reqwest::Client::new();
    let messages = json!([{ "role": "user", "content": "bonjour" }]);

    let calls = [
        (
            "/v1/chat/completions",
            json!({ "model": "chat", "messages": messages }),
        ),
        (
            "/v1/chat/completions",
            json!({ "model": "chat", "messages": messages, "stream": true }),
        ),
        (
            "/anthropic/v1/messages",
            json!({ "model": "chat", "max_tokens": 8, "messages": messages }),
        ),
        (
            "/v1/embeddings",
            json!({ "model": "chat", "input": "bonjour" }),
        ),
        (
            "/v1/chat/completions",
            json!({ "model": "inconnu", "messages": messages }),
        ),
    ];
    for (path, body) in &calls {
        // Le corps est lu en entier : un flux n'est journalisé qu'à sa fin.
        let _ = client
            .post(format!("{base}{path}"))
            .json(body)
            .send()
            .await?
            .text()
            .await?;
    }

    // Lot vidé au plus tard après 1 s (T1.18).
    let mut logs = Vec::new();
    for _ in 0..40 {
        logs = store.recent_request_logs(20).await?;
        if logs.len() >= calls.len() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let mut statuses: Vec<u16> = logs.iter().map(|log| log.status).collect();
    statuses.sort_unstable();
    assert_eq!(statuses, vec![200, 200, 200, 200, 400], "{logs:?}");
    let unknown = logs.iter().find(|log| log.status == 400);
    assert_eq!(unknown.map(|log| log.model.as_str()), Some("inconnu"));
    Ok(())
}
