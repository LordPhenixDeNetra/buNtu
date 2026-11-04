//! Preuve T1.19 : l'endpoint `/metrics` expose l'exposition Prometheus.

use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use buntu_core::config::{ConfigHandle, RawConfig};
use buntu_gateway::admin::AdminStore;
use buntu_gateway::{GatewayState, Readiness, app_with_state};

#[tokio::test]
async fn metrics_endpoint_serves_the_prometheus_exposition() -> Result<(), Box<dyn Error>> {
    buntu_observe::metrics::install_global()
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    buntu_observe::metrics::record_request(
        "mock",
        200,
        Duration::from_millis(5),
        Duration::from_micros(2),
    );

    let state = GatewayState::new(
        Readiness::new(true),
        ConfigHandle::new(buntu_core::config::CompiledConfig::default()),
    )
    .with_admin(Arc::new(AdminStore::new(RawConfig::default())));
    let router = app_with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address: SocketAddr = listener.local_addr()?;
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });

    let response = reqwest::get(format!("http://{address}/metrics")).await?;
    assert_eq!(response.status(), 200);
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    assert!(content_type.contains("text/plain"));
    let body = response.text().await?;
    assert!(body.contains("buntu_requests_total"), "exposition : {body}");
    assert!(body.contains("buntu_request_duration_seconds"));
    handle.abort();
    Ok(())
}
