//! Mode réel du kit (T1.8, étape 4) : la batterie jouée contre la vraie API d'un fournisseur.
//!
//! Activé par `BUNTU_LIVE=1`, et par fournisseur avec `BUNTU_LIVE_<NOM>_API_KEY`,
//! `BUNTU_LIVE_<NOM>_MODEL` et, au besoin, `BUNTU_LIVE_<NOM>_BASE_URL`. Une vraie réponse
//! ne reproduit jamais une fixture mot pour mot : les attentes sont donc structurelles (texte
//! non vide, raison d'arrêt, appel d'outil, JSON valide, erreur classée), pas textuelles.
//!
//! Cas non joués en réel, faute de pouvoir les provoquer à coup sûr : outils en parallèle,
//! résultat d'outil sans appel préalable, image (dépend du modèle), filtre de contenu, 400,
//! 429 et 500. Ils restent couverts par les fixtures.

use buntu_providers::provider::{Provider, StreamState};
use buntu_schema::{
    ContentPart, GatewayError, Request, ResponseFormat, StopReason, StreamEvent, ToolChoice,
};
use bytes::Bytes;

use crate::cases;
use crate::runner::live_enabled;

/// Cas joués contre la vraie API.
pub const LIVE_CASES: &[&str] = &[
    "text_simple",
    "system_multi_turn",
    "stream",
    "tool_call",
    "json_output",
    "length_stop",
    "error_401",
];

/// Service réel visé : adresse, clé (vide pour un service local sans authentification) et
/// modèle.
#[derive(Clone)]
pub struct LiveTarget {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

impl std::fmt::Debug for LiveTarget {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LiveTarget")
            .field("base_url", &self.base_url)
            .field("api_key", &"[masked]")
            .field("model", &self.model)
            .finish()
    }
}

impl LiveTarget {
    /// Lit `BUNTU_LIVE_<prefix>_*` ; `None` si le mode réel est coupé ou si le modèle manque.
    /// `default_base_url` sert quand `BASE_URL` est absente ; la clé est exigée seulement si
    /// `key_required`.
    pub fn from_env(prefix: &str, default_base_url: &str, key_required: bool) -> Option<Self> {
        if !live_enabled() {
            return None;
        }
        let var = |name: &str| {
            std::env::var(format!("BUNTU_LIVE_{prefix}_{name}"))
                .ok()
                .filter(|value| !value.is_empty())
        };
        let api_key = var("API_KEY").unwrap_or_default();
        if key_required && api_key.is_empty() {
            return None;
        }
        let base_url = var("BASE_URL").unwrap_or_else(|| default_base_url.to_owned());
        if base_url.is_empty() {
            return None;
        }
        Some(Self {
            base_url,
            api_key,
            model: var("MODEL")?,
        })
    }
}

/// Requête réelle d'un cas : celle de la batterie, adaptée pour obtenir à coup sûr le
/// comportement attendu.
fn live_request(name: &str, model: &str) -> Result<Request, String> {
    let case = cases::find(name).ok_or_else(|| format!("unknown case: {name}"))?;
    let mut request = (case.request)();
    request.model = model.to_owned();
    request.max_tokens = Some(256);
    let prompt = match name {
        "tool_call" => {
            request.tool_choice = ToolChoice::Required;
            Some("What is the weather in Dakar? Use the tool.")
        }
        "json_output" => {
            request.response_format = Some(ResponseFormat::JsonObject);
            Some("Reply with a JSON object whose only key is \"ok\", set to true.")
        }
        "length_stop" => {
            request.max_tokens = Some(5);
            Some("Write a long story about the sea.")
        }
        _ => None,
    };
    if let Some(text) = prompt
        && let Some(message) = request.messages.last_mut()
    {
        message.content = vec![ContentPart::Text {
            text: text.to_owned(),
        }];
    }
    Ok(request)
}

/// Joue un cas contre le service réel.
pub async fn run_live_case<P: Provider>(
    provider: &P,
    target: &LiveTarget,
    name: &str,
) -> Result<(), String> {
    let request = live_request(name, &target.model)?;
    let built = provider
        .build_request(&request)
        .map_err(|error| format!("build_request: {error}"))?;
    let path = built
        .uri()
        .path_and_query()
        .map(|path| path.as_str().to_owned())
        .unwrap_or_else(|| "/".to_owned());
    let url = format!("{}{path}", target.base_url.trim_end_matches('/'));

    let mut outgoing = reqwest::Client::new().post(url);
    for (header, value) in built.headers() {
        outgoing = outgoing.header(header.as_str(), value.as_bytes());
    }
    let secret = if name == "error_401" {
        "buntu-invalid-key"
    } else {
        target.api_key.as_str()
    };
    if !secret.is_empty() {
        let (header, value) = provider
            .auth_header(secret)
            .map_err(|error| format!("auth_header: {error}"))?;
        outgoing = outgoing.header(header.as_str(), value.as_bytes());
    }
    let response = outgoing
        .body(built.into_body().to_vec())
        .send()
        .await
        .map_err(|error| format!("sending: {error}"))?;
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let payload = response
        .bytes()
        .await
        .map_err(|error| format!("reading body: {error}"))?;

    if name == "error_401" {
        return match provider.map_error(status, &headers, &payload) {
            GatewayError::Authentication => Ok(()),
            other => Err(format!("status {status} mapped to {other:?}")),
        };
    }
    if status >= 400 {
        let error = provider.map_error(status, &headers, &payload);
        return Err(format!("status {status}: {error}"));
    }
    if name == "stream" {
        return check_stream(provider, payload);
    }
    let response = provider
        .parse_response(payload)
        .map_err(|error| format!("parse_response: {error}"))?;
    let message = response
        .choices
        .first()
        .map(|choice| &choice.message)
        .ok_or("no choice")?;
    let text: String = message
        .content
        .iter()
        .filter_map(|part| match part {
            ContentPart::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    let stop = response.stop_reason.clone();
    match name {
        "tool_call" => {
            let called = message
                .content
                .iter()
                .any(|part| matches!(part, ContentPart::ToolCall(call) if call.name == "meteo"));
            if !called {
                return Err(format!("no `meteo` tool call (stop {stop:?})"));
            }
            expect_stop(stop, &StopReason::ToolCalls)
        }
        "json_output" => serde_json::from_str::<serde_json::Value>(text.trim())
            .map(|_| ())
            .map_err(|error| format!("not JSON ({error}): {text}")),
        "length_stop" => expect_stop(stop, &StopReason::MaxTokens),
        _ => {
            if text.trim().is_empty() {
                return Err("empty text".to_owned());
            }
            if response.usage.input_tokens == 0 {
                return Err("usage not reported".to_owned());
            }
            expect_stop(stop, &StopReason::EndTurn)
        }
    }
}

fn expect_stop(actual: Option<StopReason>, expected: &StopReason) -> Result<(), String> {
    match actual {
        Some(reason) if &reason == expected => Ok(()),
        other => Err(format!("stop reason {other:?} instead of {expected:?}")),
    }
}

fn check_stream<P: Provider>(provider: &P, payload: Bytes) -> Result<(), String> {
    let mut state = StreamState::new();
    let mut events = Vec::new();
    // Découpage petit, comme le kit : le fournisseur doit gérer un tampon partiel.
    for chunk in payload.chunks(16) {
        events.extend(
            provider
                .parse_stream_chunk(&mut state, Bytes::copy_from_slice(chunk))
                .map_err(|error| format!("parse_stream_chunk: {error}"))?,
        );
    }
    if !state.is_finished() {
        return Err("stream did not signal its end".to_owned());
    }
    let has_text = events.iter().any(|event| {
        matches!(event, StreamEvent::ContentDelta { part: ContentPart::Text { text }, .. } if !text.is_empty())
    });
    let has_stop = events
        .iter()
        .any(|event| matches!(event, StreamEvent::Stop { .. }));
    match (has_text, has_stop) {
        (true, true) => Ok(()),
        _ => Err(format!(
            "stream without text ({has_text}) or stop ({has_stop}): {} events",
            events.len()
        )),
    }
}

/// Joue tous les cas réels et rend le résultat de chacun.
pub async fn run_live<P: Provider>(
    provider: &P,
    target: &LiveTarget,
) -> Vec<(&'static str, Result<(), String>)> {
    let mut results = Vec::new();
    for name in LIVE_CASES {
        if *name == "error_401" && target.api_key.is_empty() {
            continue; // service local sans authentification (Ollama)
        }
        results.push((*name, run_live_case(provider, target, name).await));
    }
    results
}
