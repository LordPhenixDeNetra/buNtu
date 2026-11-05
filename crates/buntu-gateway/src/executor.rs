//! Exécution des cibles : configuration → traducteur → client amont.
//!
//! Ce module remplit le contrat `TargetExecutor` du moteur (T1.6) avec le chemin réseau réel :
//! sélection de clé (T1.4), injection d'authentification, envoi hyper-util, traduction de la
//! réponse ou de l'erreur, et mise en pause de la clé sur 429.

use std::sync::Arc;

use buntu_core::config::{CompiledConfig, CompiledProvider};
use buntu_core::context::RequestContext;
use buntu_core::fallback::TargetExecutor;
use buntu_core::hooks::BoxFuture;
use buntu_core::keys::KeySelectionError;
use buntu_core::routing::Target;
use buntu_core::timeouts::{StreamTimeouts, Timeouts, with_timeout};
use buntu_providers::provider::{HttpRequest, Provider};
use buntu_schema::{EmbeddingsRequest, EmbeddingsResponse, GatewayError, Request, Response};
use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::body::Incoming;

use crate::state::GatewayState;
use crate::upstream::{MAX_ERROR_BODY_BYTES, UpstreamClient, read_limited};

/// Exécuteur HTTP adossé à un instantané de configuration.
#[derive(Clone)]
pub struct HttpExecutor {
    config: Arc<CompiledConfig>,
    registry: Arc<crate::registry::ProviderRegistry>,
    upstream: UpstreamClient,
}

/// Une requête prête à partir, avec le traducteur et la clé qui l'ont produite.
pub struct Prepared {
    pub provider: Arc<dyn Provider>,
    pub base_url: String,
    pub request: HttpRequest,
    pub key_index: usize,
}

impl HttpExecutor {
    pub fn from_state(state: &GatewayState) -> Self {
        Self {
            config: state.config.snapshot(),
            registry: Arc::clone(&state.registry),
            upstream: state.upstream.clone(),
        }
    }

    /// Timeouts applicables au modèle demandé : ceux de l'alias, sinon les globaux (T1.7).
    pub fn timeouts_for(&self, requested: &str) -> Timeouts {
        self.config
            .routes
            .get(requested)
            .map(|route| route.timeouts)
            .unwrap_or(self.config.timeouts)
    }

    fn provider_for(
        &self,
        target: &Target,
    ) -> Result<(Arc<dyn Provider>, &CompiledProvider), GatewayError> {
        let provider_config = self.config.providers.get(&target.provider).ok_or_else(|| {
            GatewayError::InvalidRequest {
                message: format!("fournisseur inconnu : `{}`", target.provider),
            }
        })?;
        let provider =
            self.registry
                .get(&provider_config.kind)
                .ok_or_else(|| GatewayError::Internal {
                    message: format!("aucun traducteur pour le type `{}`", provider_config.kind),
                })?;
        Ok((provider, provider_config))
    }

    fn pick_key(provider: &CompiledProvider) -> Result<(usize, String), GatewayError> {
        match provider.selector.select() {
            Ok(index) => {
                let key = provider
                    .keys
                    .get(index)
                    .ok_or_else(|| GatewayError::Internal {
                        message: "clé sélectionnée hors limites".to_owned(),
                    })?;
                Ok((index, key.secret.expose().to_owned()))
            }
            Err(KeySelectionError::AllPaused { retry_after }) => {
                Err(GatewayError::ProviderRateLimited {
                    retry_after: Some(retry_after),
                })
            }
            Err(other) => Err(GatewayError::Internal {
                message: other.to_string(),
            }),
        }
    }

    fn base_url(provider: &CompiledProvider) -> Result<String, GatewayError> {
        provider
            .base_url
            .clone()
            .ok_or_else(|| GatewayError::Internal {
                message: format!("`base_url` manquante pour le fournisseur `{}`", provider.id),
            })
    }

    fn authorize(
        &self,
        request: HttpRequest,
        kind: &str,
        secret: &str,
    ) -> Result<HttpRequest, GatewayError> {
        let (mut parts, body) = request.into_parts();
        let (name, value) = match kind {
            "anthropic" => (
                hyper::header::HeaderName::from_static("x-api-key"),
                secret.to_owned(),
            ),
            "gemini" => (
                hyper::header::HeaderName::from_static("x-goog-api-key"),
                secret.to_owned(),
            ),
            _ => (hyper::header::AUTHORIZATION, format!("Bearer {secret}")),
        };
        parts.headers.insert(
            name,
            hyper::header::HeaderValue::from_str(&value).map_err(|error| {
                GatewayError::Internal {
                    message: format!("en-tête d'authentification invalide : {error}"),
                }
            })?,
        );
        Ok(HttpRequest::from_parts(parts, body))
    }

    /// Prépare l'appel chat (sélection de clé + authentification incluses).
    pub fn prepare(&self, target: &Target, request: &Request) -> Result<Prepared, GatewayError> {
        let (provider, provider_config) = self.provider_for(target)?;
        let (key_index, secret) = Self::pick_key(provider_config)?;
        let built = provider.build_request(request)?;
        let request = self.authorize(built, &provider_config.kind, &secret)?;
        Ok(Prepared {
            provider,
            base_url: Self::base_url(provider_config)?,
            request,
            key_index,
        })
    }

    /// Variante embeddings.
    pub fn prepare_embeddings(
        &self,
        target: &Target,
        request: &EmbeddingsRequest,
    ) -> Result<Prepared, GatewayError> {
        let (provider, provider_config) = self.provider_for(target)?;
        let (key_index, secret) = Self::pick_key(provider_config)?;
        let built = provider.build_embeddings_request(request)?;
        let request = self.authorize(built, &provider_config.kind, &secret)?;
        Ok(Prepared {
            provider,
            base_url: Self::base_url(provider_config)?,
            request,
            key_index,
        })
    }

    /// Exécute un appel chat complet (non flux) vers la cible.
    pub async fn execute_chat(
        &self,
        target: &Target,
        request: &Request,
    ) -> Result<Response, GatewayError> {
        let prepared = self.prepare(target, request)?;
        let (status, headers, body) = self
            .upstream
            .send(&prepared.base_url, prepared.request.clone())
            .await?;
        if !(200..300).contains(&status) {
            let error = prepared.provider.map_error(status, &headers, &body);
            self.pause_on_rate_limit(target, prepared.key_index, &error);
            return Err(error);
        }
        prepared.provider.parse_response(body)
    }

    /// Ouvre un flux chat : rend le traducteur et le corps amont, sous les timeouts du flux.
    ///
    /// Les en-têtes amont doivent arriver avant le premier octet ; les morceaux suivent ensuite
    /// le budget de [`UpstreamChunks`].
    pub async fn open_stream(
        &self,
        target: &Target,
        request: &Request,
        timeouts: Timeouts,
    ) -> Result<(Arc<dyn Provider>, UpstreamChunks), GatewayError> {
        let started = std::time::Instant::now();
        let prepared = self.prepare(target, request)?;
        let (status, headers, body) = with_timeout(
            timeouts.first_byte.min(timeouts.total),
            self.upstream
                .send_stream(&prepared.base_url, prepared.request.clone()),
        )
        .await?;
        if !(200..300).contains(&status) {
            let chunk = read_limited(body, MAX_ERROR_BODY_BYTES)
                .await
                .unwrap_or_default();
            let error = prepared.provider.map_error(status, &headers, &chunk);
            self.pause_on_rate_limit(target, prepared.key_index, &error);
            return Err(error);
        }
        let chunks = UpstreamChunks {
            body,
            timeouts: StreamTimeouts::new(timeouts, started),
        };
        Ok((prepared.provider, chunks))
    }

    /// Exécute un appel embeddings vers la cible.
    pub async fn execute_embeddings(
        &self,
        target: &Target,
        request: &EmbeddingsRequest,
    ) -> Result<EmbeddingsResponse, GatewayError> {
        let prepared = self.prepare_embeddings(target, request)?;
        let (status, headers, body) = self
            .upstream
            .send(&prepared.base_url, prepared.request.clone())
            .await?;
        if !(200..300).contains(&status) {
            let error = prepared.provider.map_error(status, &headers, &body);
            self.pause_on_rate_limit(target, prepared.key_index, &error);
            return Err(error);
        }
        prepared.provider.parse_embeddings_response(body)
    }

    fn pause_on_rate_limit(&self, target: &Target, key_index: usize, error: &GatewayError) {
        if let GatewayError::ProviderRateLimited { retry_after } = error
            && let Some(delay) = retry_after
            && let Some(provider) = self.config.providers.get(&target.provider)
        {
            let _ = provider.selector.pause(key_index, *delay);
        }
    }
}

/// Corps d'un flux amont, lu morceau par morceau sous les timeouts inter-morceaux et total.
pub struct UpstreamChunks {
    body: Incoming,
    timeouts: StreamTimeouts,
}

impl UpstreamChunks {
    /// Morceau suivant ; `None` en fin de flux normale. Une coupure de connexion ou un délai
    /// dépassé rend une erreur : le client doit la voir, jamais un faux succès.
    pub async fn next(&mut self) -> Option<Result<Bytes, GatewayError>> {
        loop {
            let budget = match self.timeouts.next_budget() {
                Ok(budget) => budget,
                Err(error) => return Some(Err(error)),
            };
            match tokio::time::timeout(budget, self.body.frame()).await {
                Err(_elapsed) => return Some(Err(GatewayError::Timeout)),
                Ok(None) => return None,
                Ok(Some(Err(error))) => {
                    return Some(Err(GatewayError::ProviderUnavailable {
                        message: format!("flux amont interrompu : {error}"),
                    }));
                }
                Ok(Some(Ok(frame))) => {
                    if let Ok(data) = frame.into_data() {
                        self.timeouts.received_chunk();
                        return Some(Ok(data));
                    }
                }
            }
        }
    }
}

impl TargetExecutor for HttpExecutor {
    fn execute<'a>(
        &'a self,
        target: &'a Target,
        _context: &'a RequestContext,
        request: &'a Request,
    ) -> BoxFuture<'a, Response> {
        Box::pin(async move { self.execute_chat(target, request).await })
    }
}
