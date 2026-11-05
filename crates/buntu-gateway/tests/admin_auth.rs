//! Authentification de l'API d'administration (revue du 2026-11-05, point 1a).

use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;

use buntu_core::config::{ConfigHandle, RawConfig};
use buntu_gateway::admin::AdminStore;
use buntu_gateway::{GatewayState, Readiness, app_with_state};
use buntu_security::tokens::generate_token;
use buntu_store::Store;

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
    let router = app_with_state(state.with_admin(Arc::new(AdminStore::new(RawConfig::default()))));
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    Ok(Running { address, handle })
}

fn state() -> GatewayState {
    GatewayState::new(
        Readiness::new(true),
        ConfigHandle::new(buntu_core::config::CompiledConfig::default()),
    )
}

async fn status(
    address: SocketAddr,
    path: &str,
    token: Option<&str>,
) -> Result<u16, Box<dyn Error>> {
    let mut request = reqwest::Client::new().get(format!("http://{address}{path}"));
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    Ok(request.send().await?.status().as_u16())
}

#[tokio::test]
async fn admin_api_requires_the_configured_token() -> Result<(), Box<dyn Error>> {
    let token = generate_token();
    let running = start(state().with_admin_token(&token.token)).await?;
    let address = running.address;

    for path in [
        "/admin/v1/providers",
        "/admin/v1/routes",
        "/admin/v1/requests",
        "/admin/v1/openapi.json",
    ] {
        assert_eq!(status(address, path, None).await?, 401, "{path} sans jeton");
        assert_eq!(
            status(address, path, Some("buntu_faux")).await?,
            401,
            "{path} jeton faux"
        );
    }
    assert_eq!(
        status(address, "/admin/v1/providers", Some(&token.token)).await?,
        200
    );
    // Les routes publiques ne changent pas.
    assert_eq!(status(address, "/health", None).await?, 200);
    assert_eq!(status(address, "/v1/models", None).await?, 200);
    Ok(())
}

#[tokio::test]
async fn admin_api_without_any_token_is_closed() -> Result<(), Box<dyn Error>> {
    let running = start(state()).await?;
    assert_eq!(
        status(running.address, "/admin/v1/providers", None).await?,
        401
    );
    assert_eq!(
        status(running.address, "/admin/v1/providers", Some("buntu_x")).await?,
        401
    );
    Ok(())
}

#[tokio::test]
async fn admin_api_accepts_active_store_tokens_only() -> Result<(), Box<dyn Error>> {
    let store = Store::open_in_memory().await?;
    let active = generate_token();
    let revoked = generate_token();
    store
        .insert_api_token("actif", &active.hash, "test")
        .await?;
    store
        .insert_api_token("revoque", &revoked.hash, "test")
        .await?;
    store.revoke_api_token("revoque").await?;
    let running = start(state().with_storage(Arc::new(store))).await?;

    assert_eq!(
        status(running.address, "/admin/v1/providers", Some(&active.token)).await?,
        200
    );
    assert_eq!(
        status(running.address, "/admin/v1/providers", Some(&revoked.token)).await?,
        401
    );
    assert_eq!(
        status(running.address, "/admin/v1/providers", None).await?,
        401
    );
    Ok(())
}
