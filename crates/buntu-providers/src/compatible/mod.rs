//! Fournisseurs compatibles OpenAI (vLLM, Groq, OpenRouter, Ollama), T1.12.
//!
//! Ces serveurs exposent `/v1/chat/completions` au format OpenAI. La traduction est donc celle
//! de [`crate::openai`] ; ce module la **paramètre** (base URL, en-têtes, chemin) et déclare les
//! « quirks » connus du serveur, sans dupliquer le code OpenAI (piège de la fiche T1.12).
//!
//! Référence Ollama : <https://docs.ollama.com/api/openai-compatibility> (consultée le
//! 2026-11-04) — Ollama expose la compatibilité OpenAI ; l'API native `/api/chat` n'est pas
//! traduite dans cette tâche.

use buntu_schema::{
    Capabilities, EmbeddingsRequest, EmbeddingsResponse, GatewayError, Request, Response,
    StreamEvent,
};
use bytes::Bytes;
use hyper::HeaderMap;
use serde_json::Value;

use crate::openai::OpenAiProvider;
use crate::provider::{HttpRequest, Provider, StreamState};

pub const DEFAULT_PATH: &str = "/v1/chat/completions";

/// Serveur compatible OpenAI connu (information de documentation et de configuration).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compatible {
    Vllm,
    Groq,
    OpenRouter,
    Ollama,
}

/// Divergences déclarées d'un serveur compatible.
///
/// Aucune détection dynamique : les quirks sont **déclarés** par l'appelant, comme l'exige la
/// fiche T1.12.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quirks {
    /// Le serveur accepte `stream_options.include_usage`.
    pub stream_usage: bool,
    /// Le serveur accepte plusieurs appels d'outils en parallèle.
    pub parallel_tools: bool,
}

impl Default for Quirks {
    fn default() -> Self {
        Self {
            stream_usage: true,
            parallel_tools: true,
        }
    }
}

impl Quirks {
    pub const fn conservative() -> Self {
        Self {
            stream_usage: false,
            parallel_tools: false,
        }
    }
}

/// Traduction OpenAI réutilisée avec le chemin, la base URL, les en-têtes et les quirks.
#[derive(Debug, Clone)]
pub struct OpenAiCompatibleProvider {
    path: String,
    base_url: Option<String>,
    headers: Vec<(String, String)>,
    quirks: Quirks,
    preset: Option<Compatible>,
}

impl Default for OpenAiCompatibleProvider {
    fn default() -> Self {
        Self {
            path: DEFAULT_PATH.to_owned(),
            base_url: None,
            headers: Vec::new(),
            quirks: Quirks::default(),
            preset: None,
        }
    }
}

impl OpenAiCompatibleProvider {
    /// Serveur compatible arbitraire (chemin par défaut `/v1/chat/completions`).
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            ..Self::default()
        }
    }

    /// Préréglage ; Ollama omet `stream_options` sur certaines versions.
    pub fn preset(preset: Compatible) -> Self {
        let quirks = match preset {
            Compatible::Ollama => Quirks {
                stream_usage: false,
                ..Quirks::default()
            },
            Compatible::Vllm | Compatible::Groq | Compatible::OpenRouter => Quirks::default(),
        };
        Self {
            quirks,
            preset: Some(preset),
            ..Self::default()
        }
    }

    /// Base URL absolue (ex. `https://api.groq.com/openai`), sinon chemin relatif.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// En-tête supplémentaire (ex. OpenRouter `HTTP-Referer`, `X-Title`).
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    pub fn with_quirks(mut self, quirks: Quirks) -> Self {
        self.quirks = quirks;
        self
    }

    pub fn quirks(&self) -> Quirks {
        self.quirks
    }

    pub fn preset_name(&self) -> Option<Compatible> {
        self.preset
    }

    fn openai(&self) -> OpenAiProvider {
        let mut provider = OpenAiProvider::new(self.path.clone());
        if !self.quirks.stream_usage {
            provider = provider.without_stream_usage();
        }
        provider
    }
}

impl Provider for OpenAiCompatibleProvider {
    fn name(&self) -> &'static str {
        "openai-compatible"
    }

    fn build_request(&self, request: &Request) -> Result<HttpRequest, GatewayError> {
        let built = self.openai().build_request(request)?;
        let (mut parts, body) = built.into_parts();
        let needs_rewrite =
            self.base_url.is_some() || !self.headers.is_empty() || !self.quirks.parallel_tools;
        if !needs_rewrite {
            return Ok(HttpRequest::from_parts(parts, body));
        }

        let mut body_value: Value =
            serde_json::from_slice(&body).map_err(|error| GatewayError::Internal {
                message: format!("corps OpenAI-compatible illisible : {error}"),
            })?;
        if !self.quirks.parallel_tools
            && !request.tools.is_empty()
            && let Some(object) = body_value.as_object_mut()
        {
            object.insert("parallel_tool_calls".to_owned(), Value::Bool(false));
        }
        let bytes = serde_json::to_vec(&body_value).map_err(|error| GatewayError::Internal {
            message: format!("corps OpenAI-compatible non sérialisable : {error}"),
        })?;
        if let Some(base_url) = &self.base_url {
            let path = parts.uri.path().to_owned();
            let uri = format!("{}{path}", base_url.trim_end_matches('/'));
            parts.uri = uri.parse().map_err(|error| GatewayError::Internal {
                message: format!("URL compatible invalide : {error}"),
            })?;
        }
        for (name, value) in &self.headers {
            let header_name: hyper::header::HeaderName =
                name.parse().map_err(|error| GatewayError::Internal {
                    message: format!("en-tête compatible invalide : {error}"),
                })?;
            let header_value: hyper::header::HeaderValue =
                value.parse().map_err(|error| GatewayError::Internal {
                    message: format!("valeur d'en-tête compatible invalide : {error}"),
                })?;
            parts.headers.insert(header_name, header_value);
        }
        Ok(HttpRequest::from_parts(parts, Bytes::from(bytes)))
    }

    fn parse_response(&self, body: Bytes) -> Result<Response, GatewayError> {
        self.openai().parse_response(body)
    }

    fn parse_stream_chunk(
        &self,
        state: &mut StreamState,
        chunk: Bytes,
    ) -> Result<Vec<StreamEvent>, GatewayError> {
        self.openai().parse_stream_chunk(state, chunk)
    }

    fn map_error(&self, status: u16, headers: &HeaderMap, body: &[u8]) -> GatewayError {
        self.openai().map_error(status, headers, body)
    }

    fn capabilities(&self, model: &str) -> Capabilities {
        self.openai().capabilities(model)
    }

    fn build_embeddings_request(
        &self,
        request: &EmbeddingsRequest,
    ) -> Result<HttpRequest, GatewayError> {
        self.openai().build_embeddings_request(request)
    }

    fn parse_embeddings_response(&self, body: Bytes) -> Result<EmbeddingsResponse, GatewayError> {
        self.openai().parse_embeddings_response(body)
    }
}
