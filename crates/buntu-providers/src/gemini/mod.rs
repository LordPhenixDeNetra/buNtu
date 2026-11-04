//! Fournisseur Gemini (AI Studio), T1.11.
//!
//! Références officielles consultées le 2026-11-04 :
//! - `generateContent` : <https://ai.google.dev/api/generate-content>
//! - Streaming : <https://ai.google.dev/gemini-api/docs/streaming>
//!
//! Spécificités : rôles `user`/`model`, `systemInstruction` séparé, `functionCall` sans
//! identifiant, `finishReason` `SAFETY`/`RECITATION` pour le filtrage de contenu.

pub mod errors;
pub mod request;
pub mod response;
pub mod stream;

use buntu_schema::{Capabilities, GatewayError, Request, Response, StreamEvent};
use bytes::Bytes;
use hyper::HeaderMap;

use crate::provider::{HttpRequest, Provider, StreamState};

pub const DEFAULT_BASE: &str = "/v1beta/models";

/// Traduction Gemini `generateContent`.
#[derive(Debug, Clone)]
pub struct GeminiProvider {
    base: String,
}

impl Default for GeminiProvider {
    fn default() -> Self {
        Self {
            base: DEFAULT_BASE.to_owned(),
        }
    }
}

impl GeminiProvider {
    pub fn new(base: impl Into<String>) -> Self {
        Self { base: base.into() }
    }

    fn base(&self) -> &str {
        if self.base.is_empty() {
            DEFAULT_BASE
        } else {
            &self.base
        }
    }
}

impl Provider for GeminiProvider {
    fn name(&self) -> &'static str {
        "gemini"
    }

    fn build_request(&self, request: &Request) -> Result<HttpRequest, GatewayError> {
        request::build(self.base(), request)
    }

    fn parse_response(&self, body: Bytes) -> Result<Response, GatewayError> {
        response::parse(&body)
    }

    fn parse_stream_chunk(
        &self,
        state: &mut StreamState,
        chunk: Bytes,
    ) -> Result<Vec<StreamEvent>, GatewayError> {
        stream::parse(state, chunk)
    }

    fn map_error(&self, status: u16, headers: &HeaderMap, body: &[u8]) -> GatewayError {
        errors::map(status, headers, body)
    }

    fn capabilities(&self, _model: &str) -> Capabilities {
        // Valeur provisoire : le catalogue (T2.6) fournira les capacités par modèle.
        Capabilities {
            tools: true,
            vision: true,
        }
    }
}
