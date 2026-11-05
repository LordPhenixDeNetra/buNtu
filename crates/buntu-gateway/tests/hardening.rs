//! Durcissement du chemin de requête (revue du 2026-11-05, points 1b à 1f).
//!
//! Chaque test monte un petit amont dédié (503 permanent, amont muet, flux coupé, corps géant,
//! port non TLS) au lieu du simulateur : ces pannes ne sont pas pilotables à travers la passerelle.

use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use axum::Router;
use axum::routing::post;
use buntu_core::config::{ConfigHandle, MapEnvironment, from_str};
use buntu_gateway::{GatewayState, Readiness, app_with_state};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

/// Amont brut : chaque connexion reçoit `response` tel quel, puis la socket est fermée.
async fn spawn_raw(response: &'static [u8]) -> Result<Running, Box<dyn Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let handle = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buffer = [0_u8; 8192];
                let _ = socket.read(&mut buffer).await;
                let _ = socket.write_all(response).await;
                let _ = socket.shutdown().await;
            });
        }
    });
    Ok(Running { address, handle })
}

async fn gateway(base_url: &str, extra: &str) -> Result<Running, Box<dyn Error>> {
    let toml = format!(
        r#"
{extra}

[[providers]]
id = "up"
type = "mock"
base_url = "{base_url}"
keys = [{{ secret = "sk-test", weight = 1 }}]

[[routes]]
alias = "chat"
fallbacks = ["up/model"]
"#
    );
    let config = from_str(&toml, &MapEnvironment::default())?;
    let state = GatewayState::new(Readiness::new(true), ConfigHandle::new(config));
    spawn(app_with_state(state)).await
}

fn chat(stream: bool) -> serde_json::Value {
    json!({
        "model": "chat",
        "stream": stream,
        "messages": [{ "role": "user", "content": "bonjour" }],
    })
}

fn anthropic(stream: bool) -> serde_json::Value {
    json!({
        "model": "chat",
        "max_tokens": 16,
        "stream": stream,
        "messages": [{ "role": "user", "content": "bonjour" }],
    })
}

const CHUNK: &str = "data: {\"id\":\"c\",\"object\":\"chat.completion.chunk\",\"created\":0,\"model\":\"model\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"salut\"},\"finish_reason\":null}]}\n\n";

/// 1b : deux instances (un redémarrage) ne produisent jamais le même identifiant de requête.
#[test]
fn request_ids_do_not_repeat_across_restarts() {
    let config = ConfigHandle::new(buntu_core::config::CompiledConfig::default());
    let before = GatewayState::new(Readiness::new(true), config.clone());
    let after = GatewayState::new(Readiness::new(true), config);
    assert_ne!(before.request_id(), after.request_id());
}

/// 1c : le budget de réessais est partagé entre les requêtes (≈ 20 % plus une rafale de 20).
#[tokio::test]
async fn retry_budget_is_shared_across_requests() -> Result<(), Box<dyn Error>> {
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&hits);
    let upstream = spawn(Router::new().route(
        "/v1/chat/completions",
        post(move || {
            let counter = Arc::clone(&counter);
            async move {
                counter.fetch_add(1, Ordering::Relaxed);
                (axum::http::StatusCode::SERVICE_UNAVAILABLE, "{}")
            }
        }),
    ))
    .await?;
    let gateway = gateway(&format!("http://{}", upstream.address), "").await?;
    let client = reqwest::Client::new();
    let requests = 60;
    for _ in 0..requests {
        let response = client
            .post(format!("http://{}/v1/chat/completions", gateway.address))
            .json(&chat(false))
            .send()
            .await?;
        assert_eq!(response.status(), 502);
    }
    // Budget par requête : 3 tentatives chacune (180). Budget partagé : 60 + 20 + 60 × 0,2 = 92.
    let total = hits.load(Ordering::Relaxed);
    assert!(total <= 92, "{total} appels amont pour {requests} requêtes");
    Ok(())
}

/// 1d : un flux amont coupé en plein milieu se termine par une erreur, pas par un succès.
#[tokio::test]
async fn broken_upstream_stream_reports_an_error() -> Result<(), Box<dyn Error>> {
    // En-têtes chunked, un morceau SSE valide, puis fermeture sans le morceau final `0`.
    const BROKEN: &[u8] = b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\n\r\n\
ab\r\ndata: {\"id\":\"c\",\"object\":\"chat.completion.chunk\",\"created\":0,\"model\":\"model\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"salut\"},\"finish_reason\":null}]}\n\n\r\n";
    assert_eq!(CHUNK.len(), 0xab);
    let upstream = spawn_raw(BROKEN).await?;
    let gateway = gateway(&format!("http://{}", upstream.address), "").await?;
    let client = reqwest::Client::new();

    let openai = client
        .post(format!("http://{}/v1/chat/completions", gateway.address))
        .json(&chat(true))
        .send()
        .await?
        .text()
        .await?;
    assert!(openai.contains("salut"), "{openai}");
    assert!(
        openai.contains("\"error\""),
        "flux OpenAI sans erreur : {openai}"
    );

    let native = client
        .post(format!("http://{}/anthropic/v1/messages", gateway.address))
        .json(&anthropic(true))
        .send()
        .await?
        .text()
        .await?;
    assert!(
        native.contains("event: error"),
        "flux Anthropic sans erreur : {native}"
    );
    Ok(())
}

/// 1e : un amont muet est coupé par le timeout du premier octet (JSON et flux).
#[tokio::test]
async fn silent_upstream_hits_the_first_byte_timeout() -> Result<(), Box<dyn Error>> {
    let upstream = spawn(Router::new().route(
        "/v1/chat/completions",
        post(|| async {
            tokio::time::sleep(Duration::from_secs(60)).await;
            "{}"
        }),
    ))
    .await?;
    let gateway = gateway(
        &format!("http://{}", upstream.address),
        "[timeouts]\nfirst_byte_ms = 200",
    )
    .await?;
    let client = reqwest::Client::new();
    for stream in [false, true] {
        let started = Instant::now();
        let response = client
            .post(format!("http://{}/v1/chat/completions", gateway.address))
            .json(&chat(stream))
            .send()
            .await?;
        assert!(started.elapsed() < Duration::from_secs(10));
        let status = response.status().as_u16();
        let body = response.text().await?;
        assert!(status == 504 || status == 502, "statut {status} : {body}");
        assert!(body.contains("timeout"), "{body}");
    }
    Ok(())
}

/// 1e : un flux qui se tait après le premier morceau est coupé par l'inter-morceaux.
#[tokio::test]
async fn stalled_stream_hits_the_inter_chunk_timeout() -> Result<(), Box<dyn Error>> {
    let upstream = spawn(Router::new().route(
        "/v1/chat/completions",
        post(|| async {
            let body = async_stream::stream! {
                yield Ok::<_, std::convert::Infallible>(bytes::Bytes::from_static(CHUNK.as_bytes()));
                tokio::time::sleep(Duration::from_secs(60)).await;
            };
            (
                [(axum::http::header::CONTENT_TYPE, "text/event-stream")],
                axum::body::Body::from_stream(body),
            )
        }),
    ))
    .await?;
    let gateway = gateway(
        &format!("http://{}", upstream.address),
        "[timeouts]\ninter_chunk_ms = 200",
    )
    .await?;
    let started = Instant::now();
    let body = reqwest::Client::new()
        .post(format!("http://{}/v1/chat/completions", gateway.address))
        .json(&chat(true))
        .send()
        .await?
        .text()
        .await?;
    assert!(started.elapsed() < Duration::from_secs(10));
    assert!(body.contains("salut"), "{body}");
    assert!(body.contains("timeout"), "{body}");
    Ok(())
}

/// 1e : un corps de réponse amont démesuré est refusé au lieu d'être chargé en mémoire.
#[tokio::test]
async fn oversized_upstream_body_is_rejected() -> Result<(), Box<dyn Error>> {
    let size = buntu_gateway::upstream::MAX_RESPONSE_BYTES + 1;
    let upstream = spawn(Router::new().route(
        "/v1/chat/completions",
        post(move || async move { vec![b' '; size] }),
    ))
    .await?;
    let gateway = gateway(&format!("http://{}", upstream.address), "").await?;
    let response = reqwest::Client::new()
        .post(format!("http://{}/v1/chat/completions", gateway.address))
        .json(&chat(false))
        .send()
        .await?;
    assert_eq!(response.status(), 502);
    let body = response.text().await?;
    assert!(body.contains("trop volumineuse"), "{body}");
    Ok(())
}

/// 1f : une URL `https://` passe par la poignée de main TLS au lieu d'être refusée d'office.
#[tokio::test]
async fn https_base_url_attempts_a_tls_handshake() -> Result<(), Box<dyn Error>> {
    // Le pair parle HTTP en clair : la poignée de main TLS échoue, c'est ce qu'on veut voir.
    let upstream = spawn_raw(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n").await?;
    let gateway = gateway(&format!("https://{}", upstream.address), "").await?;
    let response = reqwest::Client::new()
        .post(format!("http://{}/v1/chat/completions", gateway.address))
        .json(&chat(false))
        .send()
        .await?;
    assert_eq!(response.status(), 502);
    let body = response.text().await?;
    assert!(!body.contains("TLS amont non branché"), "{body}");
    assert!(body.contains("connexion au fournisseur"), "{body}");
    Ok(())
}
