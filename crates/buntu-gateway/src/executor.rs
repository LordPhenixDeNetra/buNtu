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
use buntu_providers::provider::{HttpRequest, Provider};
use buntu_schema::{EmbeddingsRequest, EmbeddingsResponse, GatewayError, Request, Response};
use hyper::HeaderMap;
use hyper::body::Incoming;

use crate::state::GatewayState;
use crate::upstream::UpstreamClient;

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

    /// Ouvre un flux chat : rend le traducteur et le corps amont pour l'analyse incrémentale.
    pub async fn open_stream(
        &self,
        target: &Target,
        request: &Request,
    ) -> Result<(Arc<dyn Provider>, u16, HeaderMap, Incoming), GatewayError> {
        let prepared = self.prepare(target, request)?;
        let (status, headers, body) = self
            .upstream
            .send_stream(&prepared.base_url, prepared.request.clone())
            .await?;
        if !(200..300).contains(&status) {
            let mut chunk = Vec::new();
            use http_body_util::BodyExt;
            let mut body = body;
            while let Some(Ok(frame)) = body.frame().await {
                if let Ok(data) = frame.into_data() {
                    chunk.extend_from_slice(&data);
                }
            }
            let error = prepared.provider.map_error(status, &headers, &chunk);
            self.pause_on_rate_limit(target, prepared.key_index, &error);
            return Err(error);
        }
        Ok((prepared.provider, status, headers, body))
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
