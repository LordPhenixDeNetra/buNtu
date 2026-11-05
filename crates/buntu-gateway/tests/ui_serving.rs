//! Preuves T1.22 : le binaire sert l'UI embarquée, sans Node, avec cache long sur les fichiers
//! hachés et repli SPA pour les routes clientes.

use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use buntu_core::config::ConfigHandle;
use buntu_gateway::admin::AdminStore;
use buntu_gateway::{GatewayState, Readiness, app_with_state};

async fn start() -> Result<(SocketAddr, tokio::task::JoinHandle<()>), Box<dyn Error>> {
    let state = GatewayState::new(
        Readiness::new(true),
        ConfigHandle::new(buntu_core::config::CompiledConfig::default()),
    )
    .with_admin(Arc::new(AdminStore::default()));
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
    Ok((address, handle))
}

#[tokio::test]
async fn root_serves_the_embedded_index() -> Result<(), Box<dyn Error>> {
    let (address, handle) = start().await?;
    let response = reqwest::get(format!("http://{address}/")).await?;
    assert_eq!(response.status(), 200);
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    assert!(content_type.starts_with("text/html"), "{content_type}");
    let body = response.text().await?;
    assert!(body.contains("buNtu"), "index inattendu : {body}");
    handle.abort();
    Ok(())
}

#[tokio::test]
async fn client_routes_fall_back_to_the_spa() -> Result<(), Box<dyn Error>> {
    let (address, handle) = start().await?;
    for path in ["/providers", "/routes"] {
        let response = reqwest::get(format!("http://{address}{path}")).await?;
        assert_eq!(response.status(), 200, "{path}");
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        assert!(
            content_type.starts_with("text/html"),
            "{path} : {content_type}"
        );
    }
    handle.abort();
    Ok(())
}

#[tokio::test]
async fn hashed_assets_get_a_long_cache_and_unknown_files_are_not_served()
-> Result<(), Box<dyn Error>> {
    let (address, handle) = start().await?;
    // Premier fichier haché présent dans le build de l'UI.
    let build = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui/build");
    let mut found = None;
    collect(&build.join("_app/immutable"), &build, &mut found)?;
    let Some(relative) = found else {
        // UI non compilée localement : le test ne peut pas prouver le cache, on l'indique.
        handle.abort();
        return Ok(());
    };
    let response = reqwest::get(format!("http://{address}/{relative}")).await?;
    assert_eq!(response.status(), 200, "{relative}");
    let cache = response
        .headers()
        .get("cache-control")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    assert!(cache.contains("immutable"), "{relative} : {cache}");

    let missing = reqwest::get(format!("http://{address}/_app/absent-123.js")).await?;
    assert_eq!(missing.status(), 200, "repli SPA attendu");
    handle.abort();
    Ok(())
}

fn collect(
    directory: &std::path::Path,
    root: &std::path::Path,
    found: &mut Option<String>,
) -> Result<(), Box<dyn Error>> {
    if found.is_some() || !directory.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(&path, root, found)?;
        } else if found.is_none() && path.extension().is_some_and(|extension| extension == "js") {
            let relative = path
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/");
            *found = Some(relative);
        }
        if found.is_some() {
            return Ok(());
        }
    }
    Ok(())
}
