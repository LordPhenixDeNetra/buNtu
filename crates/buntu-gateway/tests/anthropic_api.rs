//! Tests d'intégration de l'endpoint natif Anthropic (T1.14), cible Anthropic et cible OpenAI.

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

fn config(provider_kind: &str, provider_base_url: &str) -> String {
    format!(
        r#"
[server]
listen = "127.0.0.1:8080"

[[providers]]
id = "upstream"
type = "{provider_kind}"
base_url = "{provider_base_url}"
keys = [{{ secret = "sk-test", weight = 1 }}]

[[routes]]
alias = "chat"
fallbacks = ["upstream/fake-model"]
"#
    )
}

async fn start(provider_kind: &str) -> Result<(Running, Running), Box<dyn Error>> {
    let mock = spawn(buntu_mock_provider::app()).await?;
    let config = from_str(
        &config(provider_kind, &format!("http://{}", mock.address)),
        &MapEnvironment::default(),
    )?;
    let state = GatewayState::new(Readiness::new(true), ConfigHandle::new(config));
    let gateway = spawn(app_with_state(state)).await?;
    for _ in 0..100 {
        if reqwest::get(format!("http://{}/health", gateway.address))
            .await
            .map(|response| response.status().is_success())
            .unwrap_or(false)
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    Ok((mock, gateway))
}

fn request_body() -> Value {
    json!({
        "model": "chat",
        "max_tokens": 64,
        "messages": [{ "role": "user", "content": "bonjour" }],
    })
}

#[tokio::test]
async fn anthropic_target_returns_anthropic_message() -> Result<(), Box<dyn Error>> {
    let (_mock, gateway) = start("anthropic").await?;
    let response = reqwest::Client::new()
        .post(format!("http://{}/anthropic/v1/messages", gateway.address))
        .json(&request_body())
        .send()
        .await?;

    assert_eq!(response.status(), 200);
    let provider = response
        .headers()
        .get("x-buntu-provider")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    assert_eq!(provider.as_deref(), Some("upstream"));
    let body: Value = response.json().await?;
    assert_eq!(body["type"], "message");
    assert_eq!(body["role"], "assistant");
    assert!(
        body["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .contains("token")
    );
    assert_eq!(body["usage"]["input_tokens"], 10);
    assert_eq!(body["usage"]["output_tokens"], 2);
    Ok(())
}

#[tokio::test]
async fn openai_target_is_translated_to_anthropic_format() -> Result<(), Box<dyn Error>> {
    let (_mock, gateway) = start("mock").await?;
    let response = reqwest::Client::new()
        .post(format!("http://{}/anthropic/v1/messages", gateway.address))
        .json(&request_body())
        .send()
        .await?;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await?;
    assert_eq!(body["type"], "message");
    assert!(
        body["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .contains("token")
    );
    // Le simulateur OpenAI renvoie prompt_tokens=2, completion_tokens=2.
    assert_eq!(body["usage"]["input_tokens"], 2);
    assert_eq!(body["usage"]["output_tokens"], 2);
    Ok(())
}

#[tokio::test]
async fn anthropic_stream_uses_named_events() -> Result<(), Box<dyn Error>> {
    let (_mock, gateway) = start("anthropic").await?;
    let mut body = request_body();
    body["stream"] = json!(true);
    let response = reqwest::Client::new()
        .post(format!("http://{}/anthropic/v1/messages", gateway.address))
        .json(&body)
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
    assert!(text.contains("event: message_start"), "flux : {text}");
    assert!(text.contains("event: content_block_delta"), "flux : {text}");
    assert!(text.contains("event: message_stop"), "flux : {text}");
    Ok(())
}

#[tokio::test]
async fn missing_max_tokens_returns_a_normalized_error() -> Result<(), Box<dyn Error>> {
    let (_mock, gateway) = start("anthropic").await?;
    let response = reqwest::Client::new()
        .post(format!("http://{}/anthropic/v1/messages", gateway.address))
        .json(&json!({
            "model": "chat",
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
