//! Gestionnaires de l'API d'administration (T1.16).
//!
//! Les secrets ne sortent jamais : [`KeyView::preview`] montre `sk-…a1b2` ou `env:VAR`, jamais la
//! valeur complète. Les mutations passent par [`AdminStore::mutate`] puis publient la
//! configuration compilée dans le `ConfigHandle`.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use buntu_core::config::{RawKey, RawProvider, RawRoute};
use buntu_schema::GatewayError;
use serde::{Deserialize, Serialize};
use utoipa::OpenApi;

use crate::admin::AdminApi;
use crate::error_response;
use crate::state::GatewayState;

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct KeyView {
    /// Aperçu seulement : `sk-…a1b2` ou `env:VAR`.
    pub preview: String,
    pub weight: u32,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ProviderView {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub base_url: Option<String>,
    pub keys: Vec<KeyView>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ProviderPage {
    pub object: String,
    pub data: Vec<ProviderView>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct CreateKey {
    pub secret: String,
    pub weight: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct CreateProvider {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub base_url: Option<String>,
    pub keys: Vec<CreateKey>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
pub struct UpdateProvider {
    pub base_url: Option<String>,
    pub keys: Option<Vec<CreateKey>>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct RouteView {
    pub alias: String,
    pub fallbacks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct RoutePage {
    pub object: String,
    pub data: Vec<RouteView>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct CreateRoute {
    pub alias: String,
    pub fallbacks: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
pub struct UpdateRoute {
    pub fallbacks: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
pub struct Pagination {
    pub cursor: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ErrorDetail {
    pub message: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ErrorEnvelope {
    pub error: ErrorDetail,
}

fn preview(secret: &str) -> String {
    if let Some(variable) = secret.strip_prefix("env:") {
        return format!("env:{variable}");
    }
    let chars: Vec<char> = secret.chars().collect();
    if chars.len() <= 8 {
        return "***".to_owned();
    }
    let head: String = chars[..3].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}…{tail}")
}

fn provider_view(provider: &RawProvider) -> ProviderView {
    ProviderView {
        id: provider.id.clone(),
        kind: provider.kind.clone(),
        base_url: provider.base_url.clone(),
        keys: provider
            .keys
            .iter()
            .map(|key| KeyView {
                preview: preview(&key.secret),
                weight: key.weight,
            })
            .collect(),
    }
}

fn route_view(route: &RawRoute) -> RouteView {
    RouteView {
        alias: route.alias.clone(),
        fallbacks: route.fallbacks.clone(),
    }
}

fn to_raw_keys(keys: &[CreateKey]) -> Vec<RawKey> {
    keys.iter()
        .map(|key| RawKey {
            secret: key.secret.clone(),
            weight: key.weight.unwrap_or(1),
        })
        .collect()
}

fn paginate<T: Clone>(mut items: Vec<(String, T)>, query: &Pagination) -> (Vec<T>, Option<String>) {
    items.sort_by(|left, right| left.0.cmp(&right.0));
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let start = match &query.cursor {
        Some(cursor) => items
            .iter()
            .position(|(key, _)| key.as_str() > cursor.as_str())
            .unwrap_or(items.len()),
        None => 0,
    };
    let page: Vec<(String, T)> = items.into_iter().skip(start).take(limit + 1).collect();
    let has_more = page.len() > limit;
    let mut page = page;
    if has_more {
        page.pop();
    }
    let next_cursor = if has_more {
        page.last().map(|(key, _)| key.clone())
    } else {
        None
    };
    (
        page.into_iter().map(|(_, value)| value).collect(),
        next_cursor,
    )
}

fn published(state: &GatewayState, compiled: buntu_core::config::CompiledConfig) {
    state.config.store(compiled);
}

fn created(value: impl Serialize) -> Response {
    (axum::http::StatusCode::CREATED, Json(value)).into_response()
}

fn ok(value: impl Serialize) -> Response {
    Json(value).into_response()
}

fn no_content() -> Response {
    axum::http::StatusCode::NO_CONTENT.into_response()
}

fn not_found(message: &str) -> Response {
    error_response::response(&GatewayError::InvalidRequest {
        message: message.to_owned(),
    })
}

// ---------------------------------------------------------------------------------------------
// Fournisseurs
// ---------------------------------------------------------------------------------------------

#[utoipa::path(get, path = "/admin/v1/providers",
    params(
        ("cursor" = Option<String>, Query, description = "Curseur de pagination (dernier identifiant reçu)"),
        ("limit" = Option<usize>, Query, description = "Taille de page, 1 à 100 (défaut 20)")),
    responses((status = 200, body = ProviderPage)))]
pub async fn list_providers(
    State(state): State<GatewayState>,
    Query(query): Query<Pagination>,
) -> Response {
    let raw = state.admin.snapshot();
    let items: Vec<(String, ProviderView)> = raw
        .providers
        .iter()
        .map(|provider| (provider.id.clone(), provider_view(provider)))
        .collect();
    let (data, next_cursor) = paginate(items, &query);
    ok(ProviderPage {
        object: "list".to_owned(),
        data,
        next_cursor,
    })
}

#[utoipa::path(post, path = "/admin/v1/providers", request_body = CreateProvider,
    responses((status = 201, body = ProviderView), (status = 400, body = ErrorEnvelope)))]
pub async fn create_provider(
    State(state): State<GatewayState>,
    Json(body): Json<CreateProvider>,
) -> Response {
    let id = body.id.clone();
    let provider = RawProvider {
        id: body.id.clone(),
        kind: body.kind,
        base_url: body.base_url,
        keys: to_raw_keys(&body.keys),
    };
    let compiled = match state.admin.mutate(|raw| {
        if raw.providers.iter().any(|existing| existing.id == id) {
            return Err(GatewayError::InvalidRequest {
                message: format!("le fournisseur `{id}` existe déjà"),
            });
        }
        raw.providers.push(provider.clone());
        Ok(())
    }) {
        Ok(compiled) => compiled,
        Err(error) => return error_response::response(&error),
    };
    published(&state, compiled);
    created(provider_view(&provider))
}

#[utoipa::path(get, path = "/admin/v1/providers/{id}",
    params(("id" = String, Path, description = "Identifiant du fournisseur")),
    responses((status = 200, body = ProviderView), (status = 404, body = ErrorEnvelope)))]
pub async fn get_provider(State(state): State<GatewayState>, Path(id): Path<String>) -> Response {
    let raw = state.admin.snapshot();
    match raw.providers.iter().find(|provider| provider.id == id) {
        Some(provider) => ok(provider_view(provider)),
        None => not_found(&format!("fournisseur `{id}` inconnu")),
    }
}

#[utoipa::path(patch, path = "/admin/v1/providers/{id}",
    params(("id" = String, Path, description = "Identifiant du fournisseur")),
    request_body = UpdateProvider,
    responses((status = 200, body = ProviderView), (status = 400, body = ErrorEnvelope), (status = 404, body = ErrorEnvelope)))]
pub async fn update_provider(
    State(state): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateProvider>,
) -> Response {
    let compiled = match state.admin.mutate(|raw| {
        let provider = raw
            .providers
            .iter_mut()
            .find(|provider| provider.id == id)
            .ok_or_else(|| GatewayError::InvalidRequest {
                message: format!("fournisseur `{id}` inconnu"),
            })?;
        if let Some(base_url) = &body.base_url {
            provider.base_url = Some(base_url.clone());
        }
        if let Some(keys) = &body.keys {
            provider.keys = to_raw_keys(keys);
        }
        Ok(())
    }) {
        Ok(compiled) => compiled,
        Err(error) => return error_response::response(&error),
    };
    published(&state, compiled);
    get_provider(State(state), Path(id)).await
}

#[utoipa::path(delete, path = "/admin/v1/providers/{id}",
    params(("id" = String, Path, description = "Identifiant du fournisseur")),
    responses((status = 204), (status = 404, body = ErrorEnvelope)))]
pub async fn delete_provider(
    State(state): State<GatewayState>,
    Path(id): Path<String>,
) -> Response {
    let compiled = match state.admin.mutate(|raw| {
        let before = raw.providers.len();
        raw.providers.retain(|provider| provider.id != id);
        if raw.providers.len() == before {
            return Err(GatewayError::InvalidRequest {
                message: format!("fournisseur `{id}` inconnu"),
            });
        }
        Ok(())
    }) {
        Ok(compiled) => compiled,
        Err(error) => return error_response::response(&error),
    };
    published(&state, compiled);
    no_content()
}

#[utoipa::path(post, path = "/admin/v1/providers/{id}/keys",
    params(("id" = String, Path, description = "Identifiant du fournisseur")),
    request_body = CreateKey,
    responses((status = 201, body = KeyView), (status = 400, body = ErrorEnvelope), (status = 404, body = ErrorEnvelope)))]
pub async fn create_key(
    State(state): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<CreateKey>,
) -> Response {
    let view = KeyView {
        preview: preview(&body.secret),
        weight: body.weight.unwrap_or(1),
    };
    let compiled = match state.admin.mutate(|raw| {
        let provider = raw
            .providers
            .iter_mut()
            .find(|provider| provider.id == id)
            .ok_or_else(|| GatewayError::InvalidRequest {
                message: format!("fournisseur `{id}` inconnu"),
            })?;
        provider.keys.push(RawKey {
            secret: body.secret.clone(),
            weight: body.weight.unwrap_or(1),
        });
        Ok(())
    }) {
        Ok(compiled) => compiled,
        Err(error) => return error_response::response(&error),
    };
    published(&state, compiled);
    created(view)
}

#[utoipa::path(delete, path = "/admin/v1/providers/{id}/keys/{index}",
    params(
        ("id" = String, Path, description = "Identifiant du fournisseur"),
        ("index" = usize, Path, description = "Position de la clé")),
    responses((status = 204), (status = 400, body = ErrorEnvelope), (status = 404, body = ErrorEnvelope)))]
pub async fn delete_key(
    State(state): State<GatewayState>,
    Path((id, index)): Path<(String, usize)>,
) -> Response {
    let compiled = match state.admin.mutate(|raw| {
        let provider = raw
            .providers
            .iter_mut()
            .find(|provider| provider.id == id)
            .ok_or_else(|| GatewayError::InvalidRequest {
                message: format!("fournisseur `{id}` inconnu"),
            })?;
        if index >= provider.keys.len() {
            return Err(GatewayError::InvalidRequest {
                message: format!("clé {index} inexistante pour `{id}`"),
            });
        }
        provider.keys.remove(index);
        Ok(())
    }) {
        Ok(compiled) => compiled,
        Err(error) => return error_response::response(&error),
    };
    published(&state, compiled);
    no_content()
}

// ---------------------------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------------------------

#[utoipa::path(get, path = "/admin/v1/routes",
    params(
        ("cursor" = Option<String>, Query, description = "Curseur de pagination"),
        ("limit" = Option<usize>, Query, description = "Taille de page, 1 à 100 (défaut 20)")),
    responses((status = 200, body = RoutePage)))]
pub async fn list_routes(
    State(state): State<GatewayState>,
    Query(query): Query<Pagination>,
) -> Response {
    let raw = state.admin.snapshot();
    let items: Vec<(String, RouteView)> = raw
        .routes
        .iter()
        .map(|route| (route.alias.clone(), route_view(route)))
        .collect();
    let (data, next_cursor) = paginate(items, &query);
    ok(RoutePage {
        object: "list".to_owned(),
        data,
        next_cursor,
    })
}

#[utoipa::path(post, path = "/admin/v1/routes", request_body = CreateRoute,
    responses((status = 201, body = RouteView), (status = 400, body = ErrorEnvelope)))]
pub async fn create_route(
    State(state): State<GatewayState>,
    Json(body): Json<CreateRoute>,
) -> Response {
    let alias = body.alias.clone();
    let route = RawRoute {
        alias: body.alias,
        fallbacks: body.fallbacks,
        timeouts: buntu_core::config::RawConfig::default().timeouts,
    };
    let compiled = match state.admin.mutate(|raw| {
        if raw.routes.iter().any(|existing| existing.alias == alias) {
            return Err(GatewayError::InvalidRequest {
                message: format!("la route `{alias}` existe déjà"),
            });
        }
        raw.routes.push(route.clone());
        Ok(())
    }) {
        Ok(compiled) => compiled,
        Err(error) => return error_response::response(&error),
    };
    published(&state, compiled);
    created(route_view(&route))
}

#[utoipa::path(get, path = "/admin/v1/routes/{alias}",
    params(("alias" = String, Path, description = "Alias de la route")),
    responses((status = 200, body = RouteView), (status = 404, body = ErrorEnvelope)))]
pub async fn get_route(State(state): State<GatewayState>, Path(alias): Path<String>) -> Response {
    let raw = state.admin.snapshot();
    match raw.routes.iter().find(|route| route.alias == alias) {
        Some(route) => ok(route_view(route)),
        None => not_found(&format!("route `{alias}` inconnue")),
    }
}

#[utoipa::path(patch, path = "/admin/v1/routes/{alias}",
    params(("alias" = String, Path, description = "Alias de la route")),
    request_body = UpdateRoute,
    responses((status = 200, body = RouteView), (status = 404, body = ErrorEnvelope)))]
pub async fn update_route(
    State(state): State<GatewayState>,
    Path(alias): Path<String>,
    Json(body): Json<UpdateRoute>,
) -> Response {
    let compiled = match state.admin.mutate(|raw| {
        let route = raw
            .routes
            .iter_mut()
            .find(|route| route.alias == alias)
            .ok_or_else(|| GatewayError::InvalidRequest {
                message: format!("route `{alias}` inconnue"),
            })?;
        if let Some(fallbacks) = &body.fallbacks {
            route.fallbacks = fallbacks.clone();
        }
        Ok(())
    }) {
        Ok(compiled) => compiled,
        Err(error) => return error_response::response(&error),
    };
    published(&state, compiled);
    get_route(State(state), Path(alias)).await
}

#[utoipa::path(delete, path = "/admin/v1/routes/{alias}",
    params(("alias" = String, Path, description = "Alias de la route")),
    responses((status = 204), (status = 404, body = ErrorEnvelope)))]
pub async fn delete_route(
    State(state): State<GatewayState>,
    Path(alias): Path<String>,
) -> Response {
    let compiled = match state.admin.mutate(|raw| {
        let before = raw.routes.len();
        raw.routes.retain(|route| route.alias != alias);
        if raw.routes.len() == before {
            return Err(GatewayError::InvalidRequest {
                message: format!("route `{alias}` inconnue"),
            });
        }
        Ok(())
    }) {
        Ok(compiled) => compiled,
        Err(error) => return error_response::response(&error),
    };
    published(&state, compiled);
    no_content()
}

// ---------------------------------------------------------------------------------------------
// OpenAPI
// ---------------------------------------------------------------------------------------------

#[utoipa::path(get, path = "/admin/v1/openapi.json",
    responses((status = 200, description = "Spécification OpenAPI de l'API d'administration")))]
pub async fn openapi_json() -> Response {
    Json(AdminApi::openapi()).into_response()
}
