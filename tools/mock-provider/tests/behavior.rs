//! Tests d'intégration par comportement : le simulateur est interrogé sur une vraie socket.

use std::error::Error;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

async fn spawn() -> Result<String, Box<dyn Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    tokio::spawn(async move {
        let _ = axum::serve(listener, buntu_mock_provider::app()).await;
    });
    Ok(format!("http://{address}"))
}

#[tokio::test]
async fn chat_completion_json() -> Result<(), Box<dyn Error>> {
    let base = spawn().await?;
    let response = client()
        .post(format!("{base}/v1/chat/completions"))
        .json(&json!({ "model": "any", "stream": false }))
        .send()
        .await?;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await?;
    assert_eq!(body["object"], "chat.completion");
    assert_eq!(body["model"], "mock-model");
    assert_eq!(body["choices"][0]["message"]["role"], "assistant");
    assert_eq!(body["choices"][0]["finish_reason"], "stop");
    assert_eq!(body["usage"]["total_tokens"], 4);
    Ok(())
}

#[tokio::test]
async fn chat_completion_stream() -> Result<(), Box<dyn Error>> {
    let base = spawn().await?;
    let response = client()
        .post(format!("{base}/v1/chat/completions"))
        .header("x-mock-tokens", "3")
        .json(&json!({ "stream": true }))
        .send()
        .await?;

    assert_eq!(response.status(), 200);
    assert!(
        response.headers()["content-type"]
            .to_str()?
            .starts_with("text/event-stream")
    );

    let body = response.text().await?;
    assert!(body.contains(r#""object":"chat.completion.chunk""#));
    assert!(body.ends_with("data: [DONE]\n\n"));
    // 3 morceaux de contenu + le morceau final + [DONE].
    assert_eq!(body.matches("data: ").count(), 3 + 2);
    Ok(())
}

#[tokio::test]
async fn chat_completion_latency_is_honoured() -> Result<(), Box<dyn Error>> {
    let base = spawn().await?;
    let started = Instant::now();
    let response = client()
        .post(format!("{base}/v1/chat/completions"))
        .header("x-mock-latency-ms", "200")
        .json(&json!({ "stream": false }))
        .send()
        .await?;

    assert_eq!(response.status(), 200);
    assert!(started.elapsed() >= Duration::from_millis(180));
    Ok(())
}

#[tokio::test]
async fn chat_completion_error_statuses() -> Result<(), Box<dyn Error>> {
    let base = spawn().await?;

    for status in [429_u16, 500, 503] {
        let response = client()
            .post(format!("{base}/v1/chat/completions"))
            .header("x-mock-status", status.to_string())
            .json(&json!({ "stream": false }))
            .send()
            .await?;

        assert_eq!(response.status(), status);
        if status == 429 {
            assert_eq!(response.headers()["retry-after"], "1");
        }
        let body: Value = response.json().await?;
        assert_eq!(body["error"]["type"], "mock_error");
    }
    Ok(())
}

#[tokio::test]
async fn embeddings_json() -> Result<(), Box<dyn Error>> {
    let base = spawn().await?;
    let response = client()
        .post(format!("{base}/v1/embeddings"))
        .json(&json!({ "input": "hello", "model": "any" }))
        .send()
        .await?;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await?;
    assert_eq!(body["object"], "list");
    assert_eq!(body["data"][0]["object"], "embedding");
    assert_eq!(
        body["data"][0]["embedding"].as_array().map(Vec::len),
        Some(8)
    );
    assert_eq!(body["model"], "mock-model");
    Ok(())
}

#[tokio::test]
async fn anthropic_message_json() -> Result<(), Box<dyn Error>> {
    let base = spawn().await?;
    let response = client()
        .post(format!("{base}/v1/messages"))
        .json(&json!({ "model": "any", "stream": false, "max_tokens": 16 }))
        .send()
        .await?;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await?;
    assert_eq!(body["type"], "message");
    assert_eq!(body["role"], "assistant");
    assert_eq!(body["content"][0]["type"], "text");
    assert_eq!(body["stop_reason"], "end_turn");
    assert_eq!(body["usage"]["input_tokens"], 10);
    Ok(())
}

#[tokio::test]
async fn anthropic_message_stream() -> Result<(), Box<dyn Error>> {
    let base = spawn().await?;
    let response = client()
        .post(format!("{base}/v1/messages"))
        .header("x-mock-tokens", "2")
        .json(&json!({ "stream": true, "max_tokens": 16 }))
        .send()
        .await?;

    assert_eq!(response.status(), 200);
    let body = response.text().await?;
    assert!(body.contains("event: message_start"));
    assert!(body.contains("event: content_block_delta"));
    assert!(body.contains("event: message_stop"));
    assert_eq!(body.matches("event: content_block_delta").count(), 2);
    Ok(())
}

#[tokio::test]
async fn drop_after_tokens_cuts_the_connection() -> Result<(), Box<dyn Error>> {
    let base = spawn().await?;
    let exchange = client()
        .post(format!("{base}/v1/chat/completions"))
        .header("x-mock-tokens", "10")
        .header("x-mock-drop-after-tokens", "2")
        .json(&json!({ "stream": true }))
        .send()
        .await;

    // La coupure doit être visible côté client : soit la connexion tombe avant la fin des
    // en-têtes, soit le corps se termine en erreur. Dans les deux cas, pas de fin propre.
    if let Ok(response) = exchange {
        assert_eq!(response.status(), 200);
        assert!(response.text().await.is_err());
    }
    Ok(())
}

#[tokio::test]
async fn payloads_are_deterministic() -> Result<(), Box<dyn Error>> {
    let base = spawn().await?;
    let request = json!({ "model": "any", "stream": false });

    let first = client()
        .post(format!("{base}/v1/chat/completions"))
        .json(&request)
        .send()
        .await?
        .text()
        .await?;
    let second = client()
        .post(format!("{base}/v1/chat/completions"))
        .json(&request)
        .send()
        .await?
        .text()
        .await?;

    assert_eq!(first, second);
    Ok(())
}
