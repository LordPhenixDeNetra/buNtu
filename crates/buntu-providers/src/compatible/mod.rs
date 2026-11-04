//! Fournisseurs compatibles OpenAI (vLLM, Groq, OpenRouter, Ollama), T1.12.
//!
//! Ces serveurs exposent `/v1/chat/completions` au format OpenAI. La traduction est donc celle
//! de [`crate::openai`] ; seul le chemin et quelques divergences (par exemple `stream_options`
//! refusé par certains serveurs) sont paramétrables.
//!
//! Référence Ollama : <https://docs.ollama.com/api/openai-compatibility> (consultée le
//! 2026-11-04) — Ollama expose bien la compatibilité OpenAI, c'est ce mode qui est retenu ici
//! (l'API native `/api/chat` n'est pas traduite dans cette tâche, voir la réserve de T1.12).

use buntu_schema::{Capabilities, GatewayError, Request, Response, StreamEvent};
use bytes::Bytes;
use hyper::HeaderMap;

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

/// Traduction OpenAI réutilisée avec le chemin et les options du serveur compatible.
#[derive(Debug, Clone)]
pub struct OpenAiCompatibleProvider {
    inner: OpenAiProvider,
    preset: Option<Compatible>,
}

impl Default for OpenAiCompatibleProvider {
    fn default() -> Self {
        Self {
            inner: OpenAiProvider::new(DEFAULT_PATH),
            preset: None,
        }
    }
}

impl OpenAiCompatibleProvider {
    /// Serveur compatible arbitraire (le chemin est fixé à `/v1/chat/completions`).
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            inner: OpenAiProvider::new(path),
            preset: None,
        }
    }

    /// Préréglage : Ollama refuse `stream_options` sur certaines versions, on l'omet.
    pub fn preset(preset: Compatible) -> Self {
        let inner = match preset {
            Compatible::Ollama => OpenAiProvider::new(DEFAULT_PATH).without_stream_usage(),
            Compatible::Vllm | Compatible::Groq | Compatible::OpenRouter => {
                OpenAiProvider::new(DEFAULT_PATH)
            }
        };
        Self {
            inner,
            preset: Some(preset),
        }
    }

    pub fn preset_name(&self) -> Option<Compatible> {
        self.preset
    }

    pub fn inner(&self) -> &OpenAiProvider {
        &self.inner
    }
}

impl Provider for OpenAiCompatibleProvider {
    fn name(&self) -> &'static str {
        "openai-compatible"
    }

    fn build_request(&self, request: &Request) -> Result<HttpRequest, GatewayError> {
        self.inner.build_request(request)
    }

    fn parse_response(&self, body: Bytes) -> Result<Response, GatewayError> {
        self.inner.parse_response(body)
    }

    fn parse_stream_chunk(
        &self,
        state: &mut StreamState,
        chunk: Bytes,
    ) -> Result<Vec<StreamEvent>, GatewayError> {
        self.inner.parse_stream_chunk(state, chunk)
    }

    fn map_error(&self, status: u16, headers: &HeaderMap, body: &[u8]) -> GatewayError {
        self.inner.map_error(status, headers, body)
    }

    fn capabilities(&self, model: &str) -> Capabilities {
        self.inner.capabilities(model)
    }
}
