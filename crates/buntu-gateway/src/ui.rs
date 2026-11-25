//! Interface web embarquée (T1.22).
//!
//! Les fichiers produits par `ui/` (`npm run build`) sont compilés dans le binaire par
//! `rust-embed` : le binaire release sert l'UI **sans Node installé**. Les fichiers hachés de
//! `_app/immutable/` reçoivent un cache long ; le reste est en `no-cache` (SPA). Toute route
//! inconnue renvoie `index.html` pour la navigation côté client, sauf sous les préfixes d'API
//! (`/v1`, `/admin`, `/anthropic`), qui répondent par une 404 JSON.

use axum::extract::Path;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../ui/build"]
struct Assets;

/// Page d'accueil (SPA).
pub async fn index() -> Response {
    serve("index.html")
}

/// Préfixes réservés à l'API : jamais de repli SPA (un client recevrait du HTML).
const API_PREFIXES: &[&str] = &["v1", "admin", "anthropic"];

/// Ressource statique ou repli SPA ; 404 JSON sous les préfixes d'API.
pub async fn asset(Path(path): Path<String>) -> Response {
    let first = path
        .trim_start_matches('/')
        .split('/')
        .next()
        .unwrap_or_default();
    if API_PREFIXES.contains(&first) {
        return api_not_found(&path);
    }
    serve(&path)
}

fn api_not_found(path: &str) -> Response {
    let body = serde_json::json!({
        "error": {
            "message": format!("unknown route `/{}`", path.trim_start_matches('/')),
            "type": "invalid_request_error",
            "code": "not_found",
        }
    });
    (
        StatusCode::NOT_FOUND,
        [("x-buntu-error-source", "client")],
        Json(body),
    )
        .into_response()
}

fn serve(path: &str) -> Response {
    let path = path.trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    let (served, file) = match Assets::get(path) {
        Some(file) => (path, file),
        // Repli SPA : la route cliente reçoit `index.html` (et son type MIME).
        None => match Assets::get("index.html") {
            Some(file) => ("index.html", file),
            None => {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "console not built: run `npm run build` in ui/",
                )
                    .into_response();
            }
        },
    };
    file_response(served, file)
}

fn file_response(path: &str, file: rust_embed::EmbeddedFile) -> Response {
    let mut response = file.data.into_owned().into_response();
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, content_type(path));
    let cache = if path.starts_with("_app/immutable/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    headers.insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static(cache),
    );
    response
}

fn content_type(path: &str) -> axum::http::HeaderValue {
    let extension = path.rsplit('.').next().unwrap_or_default();
    let value = match extension {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "webmanifest" => "application/manifest+json",
        "woff2" => "font/woff2",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    };
    axum::http::HeaderValue::from_static(value)
}
