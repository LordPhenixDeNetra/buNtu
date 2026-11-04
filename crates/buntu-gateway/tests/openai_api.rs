//! Tests d'intégration des endpoints OpenAI (T1.13) contre le simulateur T0.3.

use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;

use axum::Router;
use buntu_core::config::{ConfigHandle, MapEnvironment, from_str};
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

async fn spawn(router: Router) -> Result<Running, Box<dyn Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    Ok(Running { address, handle })
}

async fn spawn_mock() -> Result<Running, Box<dyn Error>> {
    spawn(buntu_mock_provider::app()).await
}

fn gateway_config(provider_base_url: &str) -> String {
    format!(
        r#"
[server]
listen = "127.0.0.1:8080"

[[providers]]
id = "mock"
type = "mock"
base_url = "{provider_base_url}"
keys = [{{ secret = "sk-test", weight = 1 }}]

[[routes]]
alias = "chat"
fallbacks = ["mock/fake-model"]
"#
    )
}

async fn spawn_gateway(provider_base_url: &str) -> Result<Running, Box<dyn Error>> {
    let config = from_str(
        &gateway_config(provider_base_url),
        &MapEnvironment::default(),
    )?;
    let state = GatewayState::new(Readiness::new(true), ConfigHandle::new(config));
    spawn(app_with_state(state)).await
}

async fn wait_healthy(address: SocketAddr) -> Result<(), Box<dyn Error>> {
    for _ in 0..100 {
        if reqwest::get(format!("http://{address}/health"))
            .await
            .map(|response| response.status().is_success())
            .unwrap_or(false)
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    Err("la passerelle n'est pas prête".into())
}

async fn start() -> Result<(Running, Running), Box<dyn Error>> {
    let mock = spawn_mock().await?;
    let gateway = spawn_gateway(&format!("http://{}", mock.address)).await?;
    wait_healthy(gateway.address).await?;
    Ok((mock, gateway))
}

#[tokio::test]
async fn chat_completions_returns_an_openai_response() -> Result<(), Box<dyn Error>> {
    let (_mock, gateway) = start().await?;
    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://{}/v1/chat/completions", gateway.address))
        .json(&json!({
            "model": "chat",
            "messages": [{ "role": "user", "content": "bonjour" }],
        }))
        .send()
        .await?;

    assert_eq!(response.status(), 200);
    let provider = response
        .headers()
        .get("x-buntu-provider")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let model = response
        .headers()
        .get("x-buntu-model")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let attempts = response
        .headers()
        .get("x-buntu-attempts")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    assert_eq!(provider.as_deref(), Some("mock"));
    assert_eq!(model.as_deref(), Some("fake-model"));
    assert_eq!(attempts.as_deref(), Some("1"));

    let body: Value = response.json().await?;
    assert_eq!(body["object"], "chat.completion");
    assert!(
        body["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .contains("token")
    );
    assert_eq!(body["usage"]["prompt_tokens"], 2);
    Ok(())
}

#[tokio::test]
async fn chat_completions_streams_sse_with_done() -> Result<(), Box<dyn Error>> {
    let (_mock, gateway) = start().await?;
    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://{}/v1/chat/completions", gateway.address))
        .json(&json!({
            "model": "chat",
            "messages": [{ "role": "user", "content": "bonjour" }],
            "stream": true,
        }))
        .send()
        .await?;

    assert_eq!(response.status(), 200);
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    assert!(
        content_type.starts_with("text/event-stream"),
        "content-type : {content_type}"
    );
    let text = response.text().await?;
    assert!(text.contains("chat.completion.chunk"), "flux : {text}");
    assert!(text.contains("\"delta\""), "flux : {text}");
    assert!(text.contains("data: [DONE]"), "flux : {text}");
    Ok(())
}

#[tokio::test]
async fn embeddings_returns_vectors() -> Result<(), Box<dyn Error>> {
    let (_mock, gateway) = start().await?;
    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://{}/v1/embeddings", gateway.address))
        .json(&json!({ "model": "chat", "input": "bonjour" }))
        .send()
        .await?;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await?;
    assert_eq!(body["object"], "list");
    let embedding = body["data"][0]["embedding"]
        .as_array()
        .ok_or("vecteur absent")?;
    assert_eq!(embedding.len(), 8, "vecteur : {embedding:?}");
    Ok(())
}

#[tokio::test]
async fn models_lists_aliases_and_targets() -> Result<(), Box<dyn Error>> {
    let (_mock, gateway) = start().await?;
    let response = reqwest::get(format!("http://{}/v1/models", gateway.address)).await?;
    assert_eq!(response.status(), 200);
    let body: Value = response.json().await?;
    let ids: Vec<String> = body["data"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry["id"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    assert!(ids.iter().any(|id| id == "chat"), "ids : {ids:?}");
    assert!(
        ids.iter().any(|id| id == "mock/fake-model"),
        "ids : {ids:?}"
    );
    Ok(())
}

#[tokio::test]
async fn unknown_model_returns_a_normalized_error() -> Result<(), Box<dyn Error>> {
    let (_mock, gateway) = start().await?;
    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://{}/v1/chat/completions", gateway.address))
        .json(&json!({
            "model": "inconnu",
            "messages": [{ "role": "user", "content": "bonjour" }],
        }))
        .send()
        .await?;

    assert_eq!(response.status(), 400);
    let source = response
        .headers()
        .get("x-buntu-error-source")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    assert_eq!(source.as_deref(), Some("client"));
    let body: Value = response.json().await?;
    assert_eq!(body["error"]["code"], "invalid_request");
    Ok(())
}

#[tokio::test]
async fn unreachable_upstream_returns_a_provider_error() -> Result<(), Box<dyn Error>> {
    // Port 1 : connexion refusée.
    let gateway = spawn_gateway("http://127.0.0.1:1").await?;
    wait_healthy(gateway.address).await?;
    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://{}/v1/chat/completions", gateway.address))
        .json(&json!({
            "model": "chat",
            "messages": [{ "role": "user", "content": "bonjour" }],
        }))
        .send()
        .await?;

    assert_eq!(response.status(), 502);
    let body: Value = response.json().await?;
    assert!(
        body["error"]["code"]
            .as_str()
            .unwrap_or_default()
            .contains("targets_failed")
            || body["error"]["code"] == "provider_unavailable",
        "erreur : {body}"
    );
    Ok(())
}
