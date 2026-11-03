//! Tests de la chaîne de repli : réessais, cible suivante, erreur finale, simulateurs.

use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;

use buntu_core::context::RequestContext;
use buntu_core::fallback::{
    FallbackChain, HEADER_ATTEMPTS, HEADER_MODEL, HEADER_PROVIDER, RoutedResponse, TargetExecutor,
};
use buntu_core::hooks::BoxFuture;
use buntu_core::retry::{Backoff, RetryBudget};
use buntu_core::routing::Target;
use buntu_schema::{GatewayError, Message, Request, Response, Role, StopReason, Usage};
use serde_json::Map;

fn sample_request() -> Request {
    Request {
        model: "chat".to_owned(),
        messages: vec![Message {
            role: Role::User,
            content: Vec::new(),
            name: None,
            extra: Map::new(),
        }],
        tools: Vec::new(),
        tool_choice: Default::default(),
        max_tokens: None,
        temperature: None,
        top_p: None,
        stop: Vec::new(),
        stream: false,
        response_format: None,
        extra: Map::new(),
    }
}

fn sample_response(id: &str) -> Response {
    Response {
        id: id.to_owned(),
        model: "modele".to_owned(),
        choices: Vec::new(),
        usage: Usage::default(),
        stop_reason: Some(StopReason::EndTurn),
        extra: Map::new(),
    }
}

fn quick_chain(max_attempts: u32) -> FallbackChain {
    FallbackChain::new(
        max_attempts,
        Backoff::new(Duration::from_millis(1), Duration::from_millis(1)),
        RetryBudget::new(20, 20),
    )
}

fn targets() -> Vec<Target> {
    vec![Target::new("a", "modele-a"), Target::new("b", "modele-b")]
}

/// Exécuteur de test : renvoie une erreur selon le statut configuré pour le fournisseur.
struct StatusExecutor {
    statuses: HashMap<String, u16>,
}

impl TargetExecutor for StatusExecutor {
    fn execute<'a>(
        &'a self,
        target: &'a Target,
        _context: &'a RequestContext,
        _request: &'a Request,
    ) -> BoxFuture<'a, Response> {
        let status = self.statuses.get(&target.provider).copied().unwrap_or(200);
        let provider = target.provider.clone();
        Box::pin(async move {
            match status {
                200 => Ok(sample_response(&provider)),
                400 => Err(GatewayError::InvalidRequest {
                    message: "400".to_owned(),
                }),
                _ => Err(GatewayError::ProviderUnavailable {
                    message: status.to_string(),
                }),
            }
        })
    }
}

#[tokio::test]
async fn falls_back_after_exhausting_retries() -> Result<(), Box<dyn Error>> {
    let executor = StatusExecutor {
        statuses: HashMap::from([("a".to_owned(), 503_u16)]),
    };
    let chain = quick_chain(2);
    let context = RequestContext::new("repli");
    let routed = chain
        .execute(&targets(), &executor, &context, &sample_request())
        .await?;

    assert_eq!(routed.provider, "b");
    assert_eq!(routed.model, "modele-b");
    assert_eq!(routed.attempts, 3, "2 tentatives sur a, 1 sur b");
    let headers: HashMap<_, _> = routed.headers().into_iter().collect();
    assert_eq!(headers.get(HEADER_PROVIDER).map(String::as_str), Some("b"));
    assert_eq!(
        headers.get(HEADER_MODEL).map(String::as_str),
        Some("modele-b")
    );
    assert_eq!(headers.get(HEADER_ATTEMPTS).map(String::as_str), Some("3"));
    Ok(())
}

#[tokio::test]
async fn non_recoverable_error_skips_retry_but_falls_back() -> Result<(), Box<dyn Error>> {
    let executor = StatusExecutor {
        statuses: HashMap::from([("a".to_owned(), 400_u16)]),
    };
    let chain = quick_chain(3);
    let context = RequestContext::new("400");
    let routed = chain
        .execute(&targets(), &executor, &context, &sample_request())
        .await?;

    assert_eq!(routed.provider, "b");
    assert_eq!(routed.attempts, 2, "aucun réessai sur un 400");
    Ok(())
}

#[tokio::test]
async fn all_targets_failed_reports_attempts_and_last_error() -> Result<(), Box<dyn Error>> {
    let executor = StatusExecutor {
        statuses: HashMap::from([("a".to_owned(), 503_u16), ("b".to_owned(), 503_u16)]),
    };
    let chain = quick_chain(2);
    let context = RequestContext::new("échec");
    let result = chain
        .execute(&targets(), &executor, &context, &sample_request())
        .await;

    match result {
        Err(GatewayError::AllTargetsFailed { attempts, last }) => {
            assert_eq!(attempts.len(), 2);
            assert!(attempts[0].contains("a/modele-a (2 tentative(s))"));
            assert!(attempts[1].contains("b/modele-b (2 tentative(s))"));
            assert!(matches!(
                last.as_ref(),
                GatewayError::ProviderUnavailable { .. }
            ));
        }
        other => panic!("erreur attendue, obtenu : {other:?}"),
    }
    Ok(())
}

#[tokio::test]
async fn exhausted_budget_stops_retrying_across_targets() -> Result<(), Box<dyn Error>> {
    let executor = StatusExecutor {
        statuses: HashMap::from([("a".to_owned(), 503_u16), ("b".to_owned(), 503_u16)]),
    };
    let chain = FallbackChain::new(
        5,
        Backoff::new(Duration::from_millis(1), Duration::from_millis(1)),
        RetryBudget::new(20, 0),
    );
    let context = RequestContext::new("budget");
    let result = chain
        .execute(&targets(), &executor, &context, &sample_request())
        .await;

    match result {
        Err(GatewayError::AllTargetsFailed { attempts, .. }) => {
            assert_eq!(attempts.len(), 2);
            assert!(
                attempts
                    .iter()
                    .all(|entry| entry.contains("1 tentative(s)"))
            );
        }
        other => panic!("erreur attendue, obtenu : {other:?}"),
    }
    Ok(())
}

// --- Scénario UC2 : deux vrais simulateurs, le premier en 503, le second OK ---------------

async fn spawn_simulator() -> Result<(SocketAddr, tokio::task::JoinHandle<()>), Box<dyn Error>> {
    let app = buntu_mock_provider::app();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Ok((address, handle))
}

struct SimulatorExecutor {
    urls: HashMap<String, String>,
    statuses: HashMap<String, u16>,
}

impl TargetExecutor for SimulatorExecutor {
    fn execute<'a>(
        &'a self,
        target: &'a Target,
        _context: &'a RequestContext,
        _request: &'a Request,
    ) -> BoxFuture<'a, Response> {
        let base = self.urls.get(&target.provider).cloned();
        let status = self.statuses.get(&target.provider).copied();
        let model = target.model.clone();
        Box::pin(async move {
            let base = base.ok_or_else(|| GatewayError::Internal {
                message: "simulateur non configuré".to_owned(),
            })?;
            let client = reqwest::Client::new();
            let mut builder =
                client
                    .post(format!("{base}/v1/chat/completions"))
                    .json(&serde_json::json!({
                        "model": model,
                        "messages": [{ "role": "user", "content": "bonjour" }],
                    }));
            if let Some(status) = status {
                builder = builder.header("x-mock-status", status.to_string());
            }
            let response =
                builder
                    .send()
                    .await
                    .map_err(|error| GatewayError::ProviderUnavailable {
                        message: error.to_string(),
                    })?;
            if response.status().is_success() {
                Ok(Response {
                    id: "simulateur".to_owned(),
                    model,
                    choices: Vec::new(),
                    usage: Usage::default(),
                    stop_reason: Some(StopReason::EndTurn),
                    extra: Map::new(),
                })
            } else {
                Err(GatewayError::ProviderUnavailable {
                    message: format!("statut {}", response.status().as_u16()),
                })
            }
        })
    }
}

#[tokio::test]
async fn uc2_first_simulator_returns_503_and_second_succeeds() -> Result<(), Box<dyn Error>> {
    let (address_a, handle_a) = spawn_simulator().await?;
    let (address_b, handle_b) = spawn_simulator().await?;

    let executor = SimulatorExecutor {
        urls: HashMap::from([
            ("sim-a".to_owned(), format!("http://{address_a}")),
            ("sim-b".to_owned(), format!("http://{address_b}")),
        ]),
        statuses: HashMap::from([("sim-a".to_owned(), 503_u16)]),
    };
    let chain = FallbackChain::new(
        2,
        Backoff::new(Duration::from_millis(1), Duration::from_millis(1)),
        RetryBudget::new(20, 20),
    );
    let targets = vec![
        Target::new("sim-a", "modele-a"),
        Target::new("sim-b", "modele-b"),
    ];
    let context = RequestContext::new("uc2");
    let routed: RoutedResponse = chain
        .execute(&targets, &executor, &context, &sample_request())
        .await?;

    assert_eq!(routed.provider, "sim-b");
    assert_eq!(routed.model, "modele-b");
    assert_eq!(routed.attempts, 3);
    assert_eq!(
        routed
            .headers()
            .iter()
            .map(|(name, value)| (*name, value.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (HEADER_PROVIDER, "sim-b"),
            (HEADER_MODEL, "modele-b"),
            (HEADER_ATTEMPTS, "3"),
        ]
    );

    handle_a.abort();
    handle_b.abort();
    Ok(())
}
