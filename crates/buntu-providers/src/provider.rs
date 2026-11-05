//! Trait commun à tous les fournisseurs (T1.8).
//!
//! Un fournisseur sait construire la requête HTTP amont, traduire la réponse ou les morceaux
//! de flux vers le format canonique, et classer une erreur HTTP. Il ne fait **aucune** I/O :
//! le client amont (hyper-util, ADR-0004) exécute la requête, le kit de conformité rejoue les
//! fixtures.

use buntu_schema::{
    Capabilities, EmbeddingsRequest, EmbeddingsResponse, GatewayError, Request, Response,
    StreamEvent,
};
use bytes::Bytes;
use hyper::HeaderMap;
use hyper::header::{HeaderName, HeaderValue};

/// Requête HTTP prête à envoyer au fournisseur.
pub type HttpRequest = hyper::Request<Bytes>;

/// État d'un flux en cours : tampon des octets reçus et suivi des événements.
#[derive(Debug, Default)]
pub struct StreamState {
    buffer: Vec<u8>,
    finished: bool,
    events: usize,
}

impl StreamState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ajoute les octets reçus du réseau au tampon.
    pub fn push(&mut self, chunk: &Bytes) {
        self.buffer.extend_from_slice(chunk);
    }

    /// Extrait les lignes complètes du tampon (le reste incomplet est conservé).
    pub fn take_lines(&mut self) -> Vec<Vec<u8>> {
        let mut lines = Vec::new();
        while let Some(position) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let mut line: Vec<u8> = self.buffer.drain(..=position).collect();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            lines.push(line);
        }
        lines
    }

    pub fn mark_finished(&mut self) {
        self.finished = true;
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn events_seen(&self) -> usize {
        self.events
    }

    pub fn record_events(&mut self, count: usize) {
        self.events = self.events.saturating_add(count);
    }
}

/// Valeur d'en-tête portant un secret, marquée sensible.
pub fn sensitive(value: &str) -> Result<HeaderValue, GatewayError> {
    let mut header = HeaderValue::from_str(value).map_err(|_| GatewayError::Internal {
        message: "clé API invalide pour un en-tête HTTP".to_owned(),
    })?;
    header.set_sensitive(true);
    Ok(header)
}

/// Traduction d'un protocole fournisseur vers le canonique.
pub trait Provider: Send + Sync {
    fn name(&self) -> &'static str;

    fn build_request(&self, request: &Request) -> Result<HttpRequest, GatewayError>;

    fn parse_response(&self, body: Bytes) -> Result<Response, GatewayError>;

    fn parse_stream_chunk(
        &self,
        state: &mut StreamState,
        chunk: Bytes,
    ) -> Result<Vec<StreamEvent>, GatewayError>;

    /// Classe une erreur HTTP en erreur canonique (le `retry-after` vient des en-têtes).
    fn map_error(&self, status: u16, headers: &HeaderMap, body: &[u8]) -> GatewayError;

    fn capabilities(&self, model: &str) -> Capabilities;

    /// En-tête d'authentification amont ; par défaut `Authorization: Bearer <clé>`.
    /// La valeur est marquée sensible (jamais affichée par `Debug`).
    fn auth_header(&self, secret: &str) -> Result<(HeaderName, HeaderValue), GatewayError> {
        Ok((
            hyper::header::AUTHORIZATION,
            sensitive(&format!("Bearer {secret}"))?,
        ))
    }

    /// Construit une requête d'embeddings ; par défaut, le fournisseur ne les prend pas en
    /// charge.
    fn build_embeddings_request(
        &self,
        _request: &EmbeddingsRequest,
    ) -> Result<HttpRequest, GatewayError> {
        Err(GatewayError::InvalidRequest {
            message: format!("{} ne prend pas en charge les embeddings", self.name()),
        })
    }

    /// Traduit une réponse d'embeddings ; par défaut, non pris en charge.
    fn parse_embeddings_response(&self, _body: Bytes) -> Result<EmbeddingsResponse, GatewayError> {
        Err(GatewayError::InvalidRequest {
            message: format!("{} ne prend pas en charge les embeddings", self.name()),
        })
    }
}
