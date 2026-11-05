//! Gestionnaires de l'API d'administration (T1.16).
//!
//! Les secrets ne sortent jamais : [`KeyView::preview`] montre `sk-…a1b2` ou `env:VAR`, jamais la
//! valeur complète. Les listes montrent la configuration effective et la **source** de chaque
//! entrée ; les mutations visent la couche base (cahier §7.1) via [`AdminStore::mutate`]. Une
//! entrée définie par le fichier est prioritaire, donc non modifiable par l'API.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use buntu_core::config::{RawConfig, RawKey, RawProvider, RawRoute};
use buntu_schema::GatewayError;
use buntu_store::{RequestLogQuery, RequestLogRecord};
use serde::{Deserialize, Serialize};
use utoipa::OpenApi;

use crate::admin::{AdminApi, Touched};
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
    /// `file` (prioritaire, lecture seule ici) ou `database` (écrit par cette API).
    pub source: String,
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
    /// `file` (prioritaire, lecture seule ici) ou `database` (écrit par cette API).
    pub source: String,
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

const SOURCE_FILE: &str = "file";
const SOURCE_DATABASE: &str = "database";

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

fn provider_view(provider: &RawProvider, source: &str) -> ProviderView {
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
        source: source.to_owned(),
    }
}

fn route_view(route: &RawRoute, source: &str) -> RouteView {
    RouteView {
        alias: route.alias.clone(),
        fallbacks: route.fallbacks.clone(),
        source: source.to_owned(),
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

/// Fournisseurs effectifs avec leur source : le fichier, puis la base sans doublon.
fn providers(state: &GatewayState) -> Vec<ProviderView> {
    let layers = state.config.layers();
    let mut views: Vec<ProviderView> = layers
        .file
        .providers
        .iter()
        .map(|provider| provider_view(provider, SOURCE_FILE))
        .collect();
    for provider in &layers.base.providers {
        if !layers
            .file
            .providers
            .iter()
            .any(|entry| entry.id == provider.id)
        {
            views.push(provider_view(provider, SOURCE_DATABASE));
        }
    }
    views
}

/// Routes effectives avec leur source.
fn routes(state: &GatewayState) -> Vec<RouteView> {
    let layers = state.config.layers();
    let mut views: Vec<RouteView> = layers
        .file
        .routes
        .iter()
        .map(|route| route_view(route, SOURCE_FILE))
        .collect();
    for route in &layers.base.routes {
        if !layers
            .file
            .routes
            .iter()
            .any(|entry| entry.alias == route.alias)
        {
            views.push(route_view(route, SOURCE_DATABASE));
        }
    }
    views
}

fn from_file(kind: &str, name: &str) -> GatewayError {
    GatewayError::InvalidRequest {
        message: format!(
            "{kind} `{name}` est défini dans le fichier de configuration, prioritaire : \
             modifiez le fichier"
        ),
    }
}

/// Fournisseur de la couche base, à modifier ; refuse une entrée du fichier.
fn base_provider<'a>(
    base: &'a mut RawConfig,
    file: &RawConfig,
    id: &str,
) -> Result<&'a mut RawProvider, GatewayError> {
    if file.providers.iter().any(|entry| entry.id == id) {
        return Err(from_file("le fournisseur", id));
    }
    base.providers
        .iter_mut()
        .find(|provider| provider.id == id)
        .ok_or_else(|| GatewayError::InvalidRequest {
            message: format!("fournisseur `{id}` inconnu"),
        })
}

/// Route de la couche base, à modifier ; refuse une entrée du fichier.
fn base_route<'a>(
    base: &'a mut RawConfig,
    file: &RawConfig,
    alias: &str,
) -> Result<&'a mut RawRoute, GatewayError> {
    if file.routes.iter().any(|entry| entry.alias == alias) {
        return Err(from_file("la route", alias));
    }
    base.routes
        .iter_mut()
        .find(|route| route.alias == alias)
        .ok_or_else(|| GatewayError::InvalidRequest {
            message: format!("route `{alias}` inconnue"),
        })
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
    let items: Vec<(String, ProviderView)> = providers(&state)
        .into_iter()
        .map(|view| (view.id.clone(), view))
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
    let result = state
        .admin
        .mutate(&state, |base, file| {
            if base.providers.iter().any(|entry| entry.id == id)
                || file.providers.iter().any(|entry| entry.id == id)
            {
                return Err(GatewayError::InvalidRequest {
                    message: format!("le fournisseur `{id}` existe déjà"),
                });
            }
            base.providers.push(provider.clone());
            Ok(Touched::Provider(id.clone()))
        })
        .await;
    match result {
        Ok(()) => created(provider_view(&provider, SOURCE_DATABASE)),
        Err(error) => error_response::response(&error),
    }
}

#[utoipa::path(get, path = "/admin/v1/providers/{id}",
    params(("id" = String, Path, description = "Identifiant du fournisseur")),
    responses((status = 200, body = ProviderView), (status = 404, body = ErrorEnvelope)))]
pub async fn get_provider(State(state): State<GatewayState>, Path(id): Path<String>) -> Response {
    match providers(&state).into_iter().find(|view| view.id == id) {
        Some(view) => ok(view),
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
    let result = state
        .admin
        .mutate(&state, |base, file| {
            let provider = base_provider(base, file, &id)?;
            if let Some(base_url) = &body.base_url {
                provider.base_url = Some(base_url.clone());
            }
            if let Some(keys) = &body.keys {
                provider.keys = to_raw_keys(keys);
            }
            Ok(Touched::Provider(id.clone()))
        })
        .await;
    match result {
        Ok(()) => get_provider(State(state), Path(id)).await,
        Err(error) => error_response::response(&error),
    }
}

#[utoipa::path(delete, path = "/admin/v1/providers/{id}",
    params(("id" = String, Path, description = "Identifiant du fournisseur")),
    responses((status = 204), (status = 404, body = ErrorEnvelope)))]
pub async fn delete_provider(
    State(state): State<GatewayState>,
    Path(id): Path<String>,
) -> Response {
    let result = state
        .admin
        .mutate(&state, |base, file| {
            base_provider(base, file, &id)?;
            base.providers.retain(|provider| provider.id != id);
            Ok(Touched::Provider(id.clone()))
        })
        .await;
    match result {
        Ok(()) => no_content(),
        Err(error) => error_response::response(&error),
    }
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
    let result = state
        .admin
        .mutate(&state, |base, file| {
            base_provider(base, file, &id)?.keys.push(RawKey {
                secret: body.secret.clone(),
                weight: body.weight.unwrap_or(1),
            });
            Ok(Touched::Provider(id.clone()))
        })
        .await;
    match result {
        Ok(()) => created(view),
        Err(error) => error_response::response(&error),
    }
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
    let result = state
        .admin
        .mutate(&state, |base, file| {
            let provider = base_provider(base, file, &id)?;
            if index >= provider.keys.len() {
                return Err(GatewayError::InvalidRequest {
                    message: format!("clé {index} inexistante pour `{id}`"),
                });
            }
            provider.keys.remove(index);
            Ok(Touched::Provider(id.clone()))
        })
        .await;
    match result {
        Ok(()) => no_content(),
        Err(error) => error_response::response(&error),
    }
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
    let items: Vec<(String, RouteView)> = routes(&state)
        .into_iter()
        .map(|view| (view.alias.clone(), view))
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
        timeouts: RawConfig::default().timeouts,
    };
    let result = state
        .admin
        .mutate(&state, |base, file| {
            if base.routes.iter().any(|entry| entry.alias == alias)
                || file.routes.iter().any(|entry| entry.alias == alias)
            {
                return Err(GatewayError::InvalidRequest {
                    message: format!("la route `{alias}` existe déjà"),
                });
            }
            base.routes.push(route.clone());
            Ok(Touched::Route(alias.clone()))
        })
        .await;
    match result {
        Ok(()) => created(route_view(&route, SOURCE_DATABASE)),
        Err(error) => error_response::response(&error),
    }
}

#[utoipa::path(get, path = "/admin/v1/routes/{alias}",
    params(("alias" = String, Path, description = "Alias de la route")),
    responses((status = 200, body = RouteView), (status = 404, body = ErrorEnvelope)))]
pub async fn get_route(State(state): State<GatewayState>, Path(alias): Path<String>) -> Response {
    match routes(&state).into_iter().find(|view| view.alias == alias) {
        Some(view) => ok(view),
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
    let result = state
        .admin
        .mutate(&state, |base, file| {
            let route = base_route(base, file, &alias)?;
            if let Some(fallbacks) = &body.fallbacks {
                route.fallbacks = fallbacks.clone();
            }
            Ok(Touched::Route(alias.clone()))
        })
        .await;
    match result {
        Ok(()) => get_route(State(state), Path(alias)).await,
        Err(error) => error_response::response(&error),
    }
}

#[utoipa::path(delete, path = "/admin/v1/routes/{alias}",
    params(("alias" = String, Path, description = "Alias de la route")),
    responses((status = 204), (status = 404, body = ErrorEnvelope)))]
pub async fn delete_route(
    State(state): State<GatewayState>,
    Path(alias): Path<String>,
) -> Response {
    let result = state
        .admin
        .mutate(&state, |base, file| {
            base_route(base, file, &alias)?;
            base.routes.retain(|route| route.alias != alias);
            Ok(Touched::Route(alias.clone()))
        })
        .await;
    match result {
        Ok(()) => no_content(),
        Err(error) => error_response::response(&error),
    }
}

// ---------------------------------------------------------------------------------------------
// Journal des requêtes (T1.24)
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct RequestLogView {
    pub id: String,
    pub model: String,
    pub provider: String,
    pub status: u16,
    pub latency_ms: u64,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub attempts: u32,
    pub created_at: i64,
}

impl From<&RequestLogRecord> for RequestLogView {
    fn from(record: &RequestLogRecord) -> Self {
        Self {
            id: record.id.clone(),
            model: record.model.clone(),
            provider: record.provider.clone(),
            status: record.status,
            latency_ms: record.latency_ms,
            input_tokens: record.input_tokens,
            output_tokens: record.output_tokens,
            attempts: record.attempts,
            created_at: record.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct RequestLogPage {
    pub object: String,
    pub data: Vec<RequestLogView>,
    pub next_cursor: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
pub struct RequestFilters {
    pub cursor: Option<i64>,
    pub limit: Option<i64>,
    pub provider: Option<String>,
    pub status: Option<u16>,
    pub model: Option<String>,
}

fn journal_unavailable() -> Response {
    error_response::response(&GatewayError::Internal {
        message: "journal non configuré (stockage indisponible)".to_owned(),
    })
}

#[utoipa::path(get, path = "/admin/v1/requests",
    params(
        ("cursor" = Option<i64>, Query, description = "Curseur de pagination (rowid)"),
        ("limit" = Option<i64>, Query, description = "Taille de page, 1 à 100 (défaut 20)"),
        ("provider" = Option<String>, Query, description = "Filtrer par fournisseur"),
        ("status" = Option<u16>, Query, description = "Filtrer par statut HTTP"),
        ("model" = Option<String>, Query, description = "Filtrer par modèle")),
    responses((status = 200, body = RequestLogPage), (status = 500, body = ErrorEnvelope)))]
pub async fn list_requests(
    State(state): State<GatewayState>,
    Query(filters): Query<RequestFilters>,
) -> Response {
    let Some(store) = state.store.clone() else {
        return journal_unavailable();
    };
    let limit = filters.limit.unwrap_or(20).clamp(1, 100);
    let query = RequestLogQuery {
        cursor: filters.cursor,
        limit,
        provider: filters.provider,
        status: filters.status,
        model: filters.model,
    };
    match store.query_request_logs(&query).await {
        Ok(records) => {
            let next_cursor = if i64::try_from(records.len()).unwrap_or(0) == limit {
                records.last().map(|record| record.rowid)
            } else {
                None
            };
            ok(RequestLogPage {
                object: "list".to_owned(),
                data: records.iter().map(RequestLogView::from).collect(),
                next_cursor,
            })
        }
        Err(error) => error_response::response(&GatewayError::Internal {
            message: error.to_string(),
        }),
    }
}

#[utoipa::path(get, path = "/admin/v1/requests/{id}",
    params(("id" = String, Path, description = "Identifiant de la requête")),
    responses((status = 200, body = RequestLogView), (status = 404, body = ErrorEnvelope)))]
pub async fn get_request(State(state): State<GatewayState>, Path(id): Path<String>) -> Response {
    let Some(store) = state.store.clone() else {
        return journal_unavailable();
    };
    match store.request_log(&id).await {
        Ok(record) => ok(RequestLogView::from(&record)),
        Err(_) => not_found(&format!("requête `{id}` inconnue")),
    }
}

// ---------------------------------------------------------------------------------------------
// OpenAPI
// ---------------------------------------------------------------------------------------------

#[utoipa::path(get, path = "/admin/v1/openapi.json",
    responses((status = 200, description = "Spécification OpenAPI de l'API d'administration")))]
pub async fn openapi_json() -> Response {
    Json(AdminApi::openapi()).into_response()
}
