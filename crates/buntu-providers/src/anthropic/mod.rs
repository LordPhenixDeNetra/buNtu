//! Fournisseur Anthropic (Messages API), T1.10.
//!
//! Références officielles consultées le 2026-11-04 :
//! - Messages : <https://docs.anthropic.com/en/api/messages>
//! - Streaming : <https://docs.anthropic.com/en/api/messages-streaming>
//!
//! Spécificités : `system` hors des messages, `max_tokens` obligatoire, blocs de contenu
//! (texte, image base64, `tool_use`, `tool_result`, `thinking`), flux à événements nommés.

pub mod errors;
pub mod request;
pub mod response;
pub mod stream;

use buntu_schema::{Capabilities, GatewayError, Request, Response, StreamEvent};
use bytes::Bytes;
use hyper::HeaderMap;

use crate::provider::{HttpRequest, Provider, StreamState};

pub const DEFAULT_PATH: &str = "/v1/messages";
pub const DEFAULT_MAX_TOKENS: u32 = 4096;

/// Traduction Anthropic Messages.
#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    path: String,
    default_max_tokens: u32,
}

impl Default for AnthropicProvider {
    fn default() -> Self {
        Self {
            path: DEFAULT_PATH.to_owned(),
            default_max_tokens: DEFAULT_MAX_TOKENS,
        }
    }
}

impl AnthropicProvider {
    pub fn new(path: impl Into<String>, default_max_tokens: u32) -> Self {
        Self {
            path: path.into(),
            default_max_tokens,
        }
    }

    fn path(&self) -> &str {
        if self.path.is_empty() {
            DEFAULT_PATH
        } else {
            &self.path
        }
    }
}

impl Provider for AnthropicProvider {
    /// `x-api-key` : <https://docs.anthropic.com/en/api/messages> (consulté le 2026-11-05).
    fn auth_header(
        &self,
        secret: &str,
    ) -> Result<(hyper::header::HeaderName, hyper::header::HeaderValue), GatewayError> {
        Ok((
            hyper::header::HeaderName::from_static("x-api-key"),
            crate::provider::sensitive(secret)?,
        ))
    }

    fn name(&self) -> &'static str {
        "anthropic"
    }

    fn build_request(&self, request: &Request) -> Result<HttpRequest, GatewayError> {
        request::build(self.path(), self.default_max_tokens, request)
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
