//! Exécution des cas de conformité : fixture rejouée par `wiremock`, puis traduction vérifiée.

use buntu_providers::provider::{Provider, StreamState};
use bytes::Bytes;

use crate::cases::{self, CaseKind, Outcome};
use crate::fixtures;

/// Joué en mode `--live` quand la variable `BUNTU_LIVE=1` est définie (utilisé par T1.9).
pub fn live_enabled() -> bool {
    std::env::var("BUNTU_LIVE")
        .map(|value| value == "1")
        .unwrap_or(false)
}

/// Exécute un cas : construit la requête, rejoue la fixture via `wiremock`, vérifie l'attente.
pub async fn run_case<P: Provider>(
    provider: &P,
    provider_dir: &str,
    name: &str,
) -> Result<(), String> {
    let case = cases::find(name).ok_or_else(|| format!("cas inconnu : {name}"))?;
    let fixture = fixtures::load(provider_dir, name)?;
    let request = (case.request)();

    let built = provider
        .build_request(&request)
        .map_err(|error| format!("build_request a échoué : {error}"))?;
    if built.method() != hyper::Method::POST {
        return Err(format!("méthode {} au lieu de POST", built.method()));
    }
    let path = built.uri().path().to_owned();
    if path.is_empty() {
        return Err("chemin de requête vide".to_owned());
    }

    let server = wiremock::MockServer::start().await;
    let mut template = wiremock::ResponseTemplate::new(fixture.status);
    for (name, value) in &fixture.headers {
        template = template.append_header(name.as_str(), value.as_str());
    }
    let template = if case.kind == CaseKind::Stream {
        template.set_body_raw(fixture.body_bytes(), "text/event-stream")
    } else {
        template.set_body_bytes(fixture.body_bytes())
    };
    wiremock::Mock::given(wiremock::matchers::any())
        .respond_with(template)
        .mount(&server)
        .await;

    let client = reqwest::Client::new();
    let mut outgoing = client.request(reqwest::Method::POST, format!("{}{}", server.uri(), path));
    for (name, value) in built.headers() {
        outgoing = outgoing.header(name.as_str(), value.as_bytes());
    }
    let body = built.into_body();
    let response = outgoing
        .body(body.to_vec())
        .send()
        .await
        .map_err(|error| format!("envoi au simulateur : {error}"))?;

    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let payload = response
        .bytes()
        .await
        .map_err(|error| format!("lecture du corps : {error}"))?;

    let outcome = match case.kind {
        CaseKind::Error => Outcome::Error(provider.map_error(status, &headers, &payload)),
        _ if status >= 400 => {
            return Err(format!("statut {status} inattendu pour un cas de succès"));
        }
        CaseKind::Stream => {
            let mut state = StreamState::new();
            let mut events = Vec::new();
            // Découpage volontairement petit : le fournisseur doit gérer un tampon partiel.
            for chunk in payload.chunks(16) {
                events.extend(
                    provider
                        .parse_stream_chunk(&mut state, Bytes::copy_from_slice(chunk))
                        .map_err(|error| format!("parse_stream_chunk : {error}"))?,
                );
            }
            if !state.is_finished() {
                return Err("le flux n'a pas signalé sa fin ([DONE])".to_owned());
            }
            Outcome::Stream(events)
        }
        CaseKind::Response => Outcome::Response(
            provider
                .parse_response(payload)
                .map_err(|error| format!("parse_response : {error}"))?,
        ),
    };

    cases::expect(name, &outcome)
}

/// Variante bloquante pour les tests générés par [`crate::conformance_suite!`].
pub fn block_on_case<P: Provider>(
    provider: &P,
    provider_dir: &str,
    name: &str,
) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?;
    runtime.block_on(run_case(provider, provider_dir, name))
}

/// Exécute toute la batterie d'un fournisseur (utilisé par T1.9 en mode `--live`).
pub async fn run_all<P: Provider>(provider: &P, provider_dir: &str) -> Result<(), String> {
    for case in cases::cases() {
        run_case(provider, provider_dir, case.name).await?;
    }
    Ok(())
}

/// Répertoire de fixtures du fournisseur, pour les appelants qui veulent l'afficher.
pub fn fixtures_of(provider_dir: &str) -> std::path::PathBuf {
    fixtures::directory(provider_dir)
}

/// Chemin d'une fixture, utile aux messages d'erreur.
pub fn fixture_path(provider_dir: &str, name: &str) -> std::path::PathBuf {
    fixtures::directory(provider_dir).join(format!("{name}.json"))
}
