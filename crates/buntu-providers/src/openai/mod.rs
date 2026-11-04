//! Fournisseur OpenAI (Chat Completions), T1.9.
//!
//! Références officielles consultées le 2026-11-03 :
//! - OpenAPI officiel (version 2.3.0) : <https://raw.githubusercontent.com/openai/openai-openapi/master/openapi.yaml>
//! - Référence : <https://platform.openai.com/docs/api-reference/chat/create>
//!
//! Le client amont (hyper-util, ADR-0004) n'existe pas encore : ce module ne fait que la
//! traduction, testée par le kit de conformité (T1.8).

pub mod errors;
pub mod request;
pub mod response;
pub mod stream;

use buntu_schema::{Capabilities, GatewayError, Request, Response, StreamEvent};
use bytes::Bytes;
use hyper::HeaderMap;

use crate::provider::{HttpRequest, Provider, StreamState};

pub const DEFAULT_PATH: &str = "/v1/chat/completions";

/// Traduction OpenAI Chat Completions.
#[derive(Debug, Clone)]
pub struct OpenAiProvider {
    path: String,
    include_stream_usage: bool,
}

impl Default for OpenAiProvider {
    fn default() -> Self {
        Self {
            path: DEFAULT_PATH.to_owned(),
            include_stream_usage: true,
        }
    }
}

impl OpenAiProvider {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            include_stream_usage: true,
        }
    }

    /// Certains serveurs compatibles OpenAI (Ollama, certains proxy) refusent
    /// `stream_options` : permet de l'omettre (T1.12).
    pub fn without_stream_usage(mut self) -> Self {
        self.include_stream_usage = false;
        self
    }

    fn path(&self) -> &str {
        if self.path.is_empty() {
            DEFAULT_PATH
        } else {
            &self.path
        }
    }
}

impl Provider for OpenAiProvider {
    fn name(&self) -> &'static str {
        "openai"
    }

    fn build_request(&self, request: &Request) -> Result<HttpRequest, GatewayError> {
        request::build(self.path(), request, self.include_stream_usage)
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
        // Valeur provisoire conservatrice : le catalogue de modèles (T2.6) fournira les
        // capacités exactes par modèle.
        Capabilities {
            tools: true,
            vision: false,
        }
    }
}
