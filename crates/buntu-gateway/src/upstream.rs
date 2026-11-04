//! Client HTTP amont (ADR-0004) : hyper-util, pool HTTP/1.1, sans TLS pour l'instant.
//!
//! Le TLS (`rustls`) sera branché avec les secrets (T1.21) : les URLs `https://` sont refusées
//! explicitement plutôt que de produire une erreur de connexion obscure.

use buntu_providers::provider::HttpRequest;
use buntu_schema::GatewayError;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::HeaderMap;
use hyper::body::Incoming;
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;

/// Client amont partagé par toutes les requêtes.
#[derive(Debug, Clone)]
pub struct UpstreamClient {
    inner: Client<HttpConnector, Full<Bytes>>,
}

impl Default for UpstreamClient {
    fn default() -> Self {
        Self::new()
    }
}

impl UpstreamClient {
    pub fn new() -> Self {
        let connector = HttpConnector::new();
        Self {
            inner: Client::builder(TokioExecutor::new()).build(connector),
        }
    }

    /// Envoie une requête et lit toute la réponse.
    pub async fn send(
        &self,
        base_url: &str,
        request: HttpRequest,
    ) -> Result<(u16, HeaderMap, Bytes), GatewayError> {
        let request = absolute_request(base_url, request)?;
        let response = self
            .inner
            .request(request.map(Full::new))
            .await
            .map_err(connection_error)?;
        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let body = response
            .into_body()
            .collect()
            .await
            .map_err(|error| GatewayError::ProviderUnavailable {
                message: format!("lecture de la réponse amont : {error}"),
            })?
            .to_bytes();
        Ok((status, headers, body))
    }

    /// Envoie une requête et rend le corps en flux ; utilisé par les réponses SSE.
    pub async fn send_stream(
        &self,
        base_url: &str,
        request: HttpRequest,
    ) -> Result<(u16, HeaderMap, Incoming), GatewayError> {
        let request = absolute_request(base_url, request)?;
        let response = self
            .inner
            .request(request.map(Full::new))
            .await
            .map_err(connection_error)?;
        let status = response.status().as_u16();
        let headers = response.headers().clone();
        Ok((status, headers, response.into_body()))
    }
}

fn connection_error(error: hyper_util::client::legacy::Error) -> GatewayError {
    GatewayError::ProviderUnavailable {
        message: format!("connexion au fournisseur : {error}"),
    }
}

fn absolute_request(base_url: &str, request: HttpRequest) -> Result<HttpRequest, GatewayError> {
    let base = base_url.trim_end_matches('/');
    if base.starts_with("https://") {
        return Err(GatewayError::ProviderUnavailable {
            message: "TLS amont non branché (T1.21) : utilisez une URL http:// pour l'instant"
                .to_owned(),
        });
    }
    if !base.starts_with("http://") {
        return Err(GatewayError::ProviderUnavailable {
            message: format!("base_url invalide (http:// attendu) : {base}"),
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
