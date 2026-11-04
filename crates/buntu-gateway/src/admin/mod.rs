//! API d'administration REST v1 (T1.16).
//!
//! CRUD fournisseurs/clés/routes sur la configuration brute, validée par le **même code** que
//! `buntu config validate` ; pagination par curseur ; les secrets ne sont **jamais** renvoyés en
//! clair (aperçu seulement). OpenAPI généré par `utoipa`, vérifié en CI (test de diff).

pub mod handlers;
pub mod store;

pub use store::AdminStore;

/// Spécification OpenAPI de l'API d'administration.
#[derive(utoipa::OpenApi)]
#[openapi(
    info(
        title = "buNtu Admin API",
        version = "1.0.0",
        description = "Configuration runtime de buNtu : fournisseurs, clés et routes."
    ),
    paths(
        handlers::openapi_json,
        handlers::list_providers,
        handlers::create_provider,
        handlers::get_provider,
        handlers::update_provider,
        handlers::delete_provider,
        handlers::create_key,
        handlers::delete_key,
        handlers::list_routes,
        handlers::create_route,
        handlers::get_route,
        handlers::update_route,
        handlers::delete_route,
    ),
    components(schemas(
        handlers::KeyView,
        handlers::ProviderView,
        handlers::ProviderPage,
        handlers::CreateKey,
        handlers::CreateProvider,
        handlers::UpdateProvider,
        handlers::RouteView,
        handlers::RoutePage,
        handlers::CreateRoute,
        handlers::UpdateRoute,
        handlers::ErrorDetail,
        handlers::ErrorEnvelope,
    ))
)]
pub struct AdminApi;
