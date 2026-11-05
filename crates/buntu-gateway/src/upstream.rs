//! Client HTTP amont (ADR-0004) : hyper-util, pool HTTP/1.1, TLS `rustls` (racines Mozilla).
//!
//! Références officielles consultées le 2026-11-05 :
//! - hyper-rustls 0.27, `HttpsConnectorBuilder::with_provider_and_webpki_roots` :
//!   <https://docs.rs/hyper-rustls/0.27/hyper_rustls/struct.HttpsConnectorBuilder.html>
//! - rustls 0.23, fournisseur `ring` : <https://docs.rs/rustls/0.23/rustls/crypto/ring/>
//!
//! Les racines CA sont embarquées (`webpki-roots`) : le binaire musl statique et l'image
//! distroless n'ont pas besoin du magasin de certificats du système.

use std::sync::Arc;

use buntu_core::timeouts::Timeouts;
use buntu_providers::provider::HttpRequest;
use buntu_schema::GatewayError;
use bytes::Bytes;
use http_body_util::{BodyExt, Full, Limited};
use hyper::HeaderMap;
use hyper::body::Incoming;
use hyper_rustls::HttpsConnector;
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;

/// Taille maximale d'une réponse amont lue en entier (JSON, embeddings) : 32 Mio.
pub const MAX_RESPONSE_BYTES: usize = 32 * 1024 * 1024;
/// Taille maximale du corps d'erreur lu à l'ouverture d'un flux : 64 Kio.
pub const MAX_ERROR_BODY_BYTES: usize = 64 * 1024;

type HttpsClient = Client<HttpsConnector<HttpConnector>, Full<Bytes>>;

/// Client amont partagé par toutes les requêtes.
#[derive(Debug, Clone)]
pub struct UpstreamClient {
    /// `Err` si la configuration TLS n'a pas pu être construite : chaque appel le signale
    /// au lieu de faire paniquer le démarrage.
    inner: Result<HttpsClient, Arc<str>>,
}

impl Default for UpstreamClient {
    fn default() -> Self {
        Self::new()
    }
}

impl UpstreamClient {
    pub fn new() -> Self {
        Self {
            inner: build_client().map_err(Arc::from),
        }
    }

    /// Envoie une requête et lit toute la réponse, dans la limite de `MAX_RESPONSE_BYTES`.
    pub async fn send(
        &self,
        base_url: &str,
        request: HttpRequest,
    ) -> Result<(u16, HeaderMap, Bytes), GatewayError> {
        let (status, headers, body) = self.send_stream(base_url, request).await?;
        let body = read_limited(body, MAX_RESPONSE_BYTES).await?;
        Ok((status, headers, body))
    }

    /// Envoie une requête et rend le corps en flux ; utilisé par les réponses SSE.
    pub async fn send_stream(
        &self,
        base_url: &str,
        request: HttpRequest,
    ) -> Result<(u16, HeaderMap, Incoming), GatewayError> {
        let client = self
            .inner
            .as_ref()
            .map_err(|error| GatewayError::Internal {
                message: format!("client TLS indisponible : {error}"),
            })?;
        let request = absolute_request(base_url, request)?;
        let response = client
            .request(request.map(Full::new))
            .await
            .map_err(connection_error)?;
        let status = response.status().as_u16();
        let headers = response.headers().clone();
        Ok((status, headers, response.into_body()))
    }
}

/// Lit un corps amont en entier sans dépasser `limit` octets.
pub async fn read_limited(body: Incoming, limit: usize) -> Result<Bytes, GatewayError> {
    match Limited::new(body, limit).collect().await {
        Ok(collected) => Ok(collected.to_bytes()),
        Err(error) if error.is::<http_body_util::LengthLimitError>() => {
            Err(GatewayError::ProviderUnavailable {
                message: format!("réponse amont trop volumineuse (> {limit} octets)"),
            })
        }
        Err(error) => Err(GatewayError::ProviderUnavailable {
            message: format!("lecture de la réponse amont : {error}"),
        }),
    }
}

fn build_client() -> Result<HttpsClient, String> {
    let mut http = HttpConnector::new();
    // Les URL `https://` sont confiées à rustls par le connecteur englobant.
    http.enforce_http(false);
    // ponytail: délai de connexion global (défaut 5 s) ; la surcharge par route de
    // `connect_ms` demanderait un client par valeur distincte.
    http.set_connect_timeout(Some(Timeouts::default().connect));
    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_provider_and_webpki_roots(rustls::crypto::ring::default_provider())
        .map_err(|error| error.to_string())?
        .https_or_http()
        .enable_http1()
        .wrap_connector(http);
    Ok(Client::builder(TokioExecutor::new()).build(https))
}

fn connection_error(error: hyper_util::client::legacy::Error) -> GatewayError {
    use std::error::Error as _;
    let mut message = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        message.push_str(" : ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    GatewayError::ProviderUnavailable {
        message: format!("connexion au fournisseur : {message}"),
    }
}

fn absolute_request(base_url: &str, request: HttpRequest) -> Result<HttpRequest, GatewayError> {
    let base = base_url.trim_end_matches('/');
    if !base.starts_with("http://") && !base.starts_with("https://") {
        return Err(GatewayError::ProviderUnavailable {
            message: format!("base_url invalide (http:// ou https:// attendu) : {base}"),
        });
    }
    let path = request
        .uri()
        .path_and_query()
        .map(|path| path.as_str())
        .unwrap_or("/");
    let uri = format!("{base}{path}");
    let (mut parts, body) = request.into_parts();
    parts.uri = uri
        .parse()
        .map_err(|error| GatewayError::ProviderUnavailable {
            message: format!("URL amont invalide : {error}"),
        })?;
    Ok(HttpRequest::from_parts(parts, body))
}
