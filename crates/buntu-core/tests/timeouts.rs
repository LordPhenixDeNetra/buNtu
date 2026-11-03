//! Tests des quatre timeouts et de leur configuration.

use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use buntu_core::config::{CompiledConfig, ConfigError, MapEnvironment, from_str};
use buntu_core::context::RequestContext;
use buntu_core::fallback::TargetExecutor;
use buntu_core::hooks::BoxFuture;
use buntu_core::routing::Target;
use buntu_core::timeouts::{StreamTimeouts, TimedExecutor, Timeouts, with_timeout};
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

fn sample_response() -> Response {
    Response {
        id: "test".to_owned(),
        model: "modele".to_owned(),
        choices: Vec::new(),
        usage: Usage::default(),
        stop_reason: Some(StopReason::EndTurn),
        extra: Map::new(),
    }
}

fn target() -> Target {
    Target::new("sim", "modele")
}

fn expect_error(result: Result<CompiledConfig, ConfigError>) -> ConfigError {
    match result {
        Ok(_) => panic!("une erreur de configuration était attendue"),
        Err(error) => error,
    }
}

/// Exécuteur lent : dort `delay` avant de répondre.
struct SlowExecutor {
    delay: Duration,
}

impl TargetExecutor for SlowExecutor {
    fn execute<'a>(
        &'a self,
        _target: &'a Target,
        _context: &'a RequestContext,
        _request: &'a Request,
    ) -> BoxFuture<'a, Response> {
        let delay = self.delay;
        Box::pin(async move {
            tokio::time::sleep(delay).await;
            Ok(sample_response())
        })
    }
}

/// Marque sa destruction : prouve que le futur abandonné par un timeout est bien détruit.
struct DropFlag(Arc<AtomicBool>);

impl Drop for DropFlag {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

#[test]
fn defaults_match_the_specification() {
    let timeouts = Timeouts::default();
    assert_eq!(timeouts.connect, Duration::from_secs(5));
    assert_eq!(timeouts.first_byte, Duration::from_secs(60));
    assert_eq!(timeouts.inter_chunk, Duration::from_secs(30));
    assert_eq!(timeouts.total, Duration::from_secs(600));
}

#[test]
fn overrides_only_replace_provided_fields() {
    let base = Timeouts::default();
    let changed = base.apply(
        None,
        Some(Duration::from_secs(300)),
        None,
        Some(Duration::from_secs(1200)),
    );
    assert_eq!(changed.connect, Duration::from_secs(5));
    assert_eq!(changed.first_byte, Duration::from_secs(300));
    assert_eq!(changed.inter_chunk, Duration::from_secs(30));
    assert_eq!(changed.total, Duration::from_secs(1200));
}

#[tokio::test]
async fn timeout_expires_and_drops_the_upstream_future() -> Result<(), Box<dyn Error>> {
    let dropped = Arc::new(AtomicBool::new(false));
    let guard = DropFlag(Arc::clone(&dropped));

    let result = with_timeout(Duration::from_millis(20), async move {
        let _guard = guard;
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok::<(), GatewayError>(())
    })
    .await;

    assert!(matches!(result, Err(GatewayError::Timeout)));
    assert!(
        dropped.load(Ordering::SeqCst),
        "le futur amont n'a pas été abandonné"
    );
    Ok(())
}

#[tokio::test]
async fn timed_executor_enforces_the_first_byte_timeout() -> Result<(), Box<dyn Error>> {
    let slow = SlowExecutor {
        delay: Duration::from_millis(200),
    };
    let timeouts = Timeouts::default().apply(None, Some(Duration::from_millis(20)), None, None);
    let executor = TimedExecutor::new(&slow, timeouts);
    let context = RequestContext::new("premier-octet");
    let result = executor
        .execute(&target(), &context, &sample_request())
        .await;

    assert!(matches!(result, Err(GatewayError::Timeout)));
    Ok(())
}

#[tokio::test]
async fn timed_executor_enforces_the_total_timeout() -> Result<(), Box<dyn Error>> {
    let slow = SlowExecutor {
        delay: Duration::from_millis(200),
    };
    let timeouts = Timeouts::default().apply(
        None,
        Some(Duration::from_secs(1)),
        None,
        Some(Duration::from_millis(20)),
    );
    let executor = TimedExecutor::new(&slow, timeouts);
    let context = RequestContext::new("total");
    let result = executor
        .execute(&target(), &context, &sample_request())
        .await;

    assert!(matches!(result, Err(GatewayError::Timeout)));
    Ok(())
}

#[tokio::test]
async fn fast_response_passes_within_the_timeouts() -> Result<(), Box<dyn Error>> {
    let fast = SlowExecutor {
        delay: Duration::from_millis(1),
    };
    let timeouts = Timeouts::default().apply(
        None,
        Some(Duration::from_millis(100)),
        None,
        Some(Duration::from_millis(200)),
    );
    let executor = TimedExecutor::new(&fast, timeouts);
    let context = RequestContext::new("rapide");
    let response = executor
        .execute(&target(), &context, &sample_request())
        .await?;
    assert_eq!(response.id, "test");
    Ok(())
}

#[test]
fn stream_budgets_follow_first_chunk_then_inter_chunk_then_total() -> Result<(), Box<dyn Error>> {
    let timeouts = Timeouts::default().apply(
        None,
        Some(Duration::from_secs(60)),
        Some(Duration::from_secs(30)),
        Some(Duration::from_secs(600)),
    );
    let started = Instant::now();
    let mut stream = StreamTimeouts::new(timeouts, started);

    assert_eq!(stream.next_budget()?, Duration::from_secs(60));
    stream.received_chunk();
    assert_eq!(stream.next_budget()?, Duration::from_secs(30));

    // Le plafond total prime sur l'inter-morceaux.
    let capped = StreamTimeouts::new(timeouts, Instant::now() - Duration::from_secs(590));
    let budget = capped.next_budget()?;
    assert!(budget <= Duration::from_secs(10), "budget {budget:?}");

    // Passé le total, plus aucun morceau n'est accepté.
    let expired = StreamTimeouts::new(timeouts, Instant::now() - Duration::from_secs(601));
    assert!(matches!(expired.next_budget(), Err(GatewayError::Timeout)));
    Ok(())
}

#[test]
fn configuration_defaults_and_route_overrides() -> Result<(), Box<dyn Error>> {
    let content = r#"
[server]
listen = "127.0.0.1:8080"

[timeouts]
connect_ms = 2500
total_ms = 900000

[[providers]]
id = "a"
type = "openai"
keys = [{ secret = "env:KEY", weight = 1 }]

[[routes]]
alias = "reasoning"
fallbacks = ["a/o1"]
timeouts = { first_byte_ms = 300000 }
"#;
    let config = from_str(content, &MapEnvironment::new([("KEY", "sk-test")]))?;

    assert_eq!(config.timeouts.connect, Duration::from_millis(2500));
    assert_eq!(config.timeouts.first_byte, Duration::from_secs(60));
    assert_eq!(config.timeouts.total, Duration::from_millis(900000));

    let route = config.routes.get("reasoning").ok_or("route absente")?;
    assert_eq!(route.timeouts.first_byte, Duration::from_millis(300000));
    assert_eq!(route.timeouts.connect, Duration::from_millis(2500));
    assert_eq!(route.timeouts.inter_chunk, Duration::from_secs(30));
    Ok(())
}

#[test]
fn zero_timeouts_are_rejected_with_their_path() {
    let global = r#"
[server]
listen = "127.0.0.1:8080"
[timeouts]
total_ms = 0
"#;
    let error = expect_error(from_str(global, &MapEnvironment::default()));
    assert!(error.to_string().contains("timeouts.total_ms"));

    let route = r#"
[server]
listen = "127.0.0.1:8080"
[[providers]]
id = "a"
type = "openai"
keys = [{ secret = "env:KEY", weight = 1 }]
[[routes]]
alias = "r"
fallbacks = ["a/m"]
timeouts = { first_byte_ms = 0 }
"#;
    let error = expect_error(from_str(route, &MapEnvironment::new([("KEY", "sk-test")])));
    assert!(
        error
            .to_string()
            .contains("routes[0].timeouts.first_byte_ms"),
        "message : {error}"
    );
}

// --- Preuve HTTP réelle : le simulateur répond après 200 ms, le premier octet est coupé à 50 ms.

async fn spawn_simulator()
-> Result<(std::net::SocketAddr, tokio::task::JoinHandle<()>), Box<dyn Error>> {
    let app = buntu_mock_provider::app();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Ok((address, handle))
}

struct LatencyExecutor {
    base: String,
    latency_ms: u64,
}

impl TargetExecutor for LatencyExecutor {
    fn execute<'a>(
        &'a self,
        _target: &'a Target,
        _context: &'a RequestContext,
        _request: &'a Request,
    ) -> BoxFuture<'a, Response> {
        let base = self.base.clone();
        let latency = self.latency_ms;
        Box::pin(async move {
            let client = reqwest::Client::new();
            let response = client
                .post(format!("{base}/v1/chat/completions"))
                .header("x-mock-latency-ms", latency.to_string())
                .json(&serde_json::json!({
                    "model": "modele",
                    "messages": [{ "role": "user", "content": "bonjour" }],
                }))
                .send()
                .await
                .map_err(|error| GatewayError::ProviderUnavailable {
                    message: error.to_string(),
                })?;
            if response.status().is_success() {
                Ok(sample_response())
            } else {
                Err(GatewayError::ProviderUnavailable {
                    message: format!("statut {}", response.status().as_u16()),
                })
            }
        })
    }
}

#[tokio::test]
async fn simulator_latency_exceeds_the_first_byte_timeout() -> Result<(), Box<dyn Error>> {
    let (address, handle) = spawn_simulator().await?;
    let executor = LatencyExecutor {
        base: format!("http://{address}"),
        latency_ms: 200,
    };
    let timeouts = Timeouts::default().apply(
        None,
        Some(Duration::from_millis(50)),
        None,
        Some(Duration::from_secs(2)),
    );
    let timed = TimedExecutor::new(&executor, timeouts);
    let context = RequestContext::new("simulateur-lent");
    let started = Instant::now();
    let result = timed.execute(&target(), &context, &sample_request()).await;
    let elapsed = started.elapsed();

    assert!(
        matches!(result, Err(GatewayError::Timeout)),
        "résultat : {result:?}"
    );
    assert!(
        elapsed >= Duration::from_millis(45),
        "timeout trop tôt : {elapsed:?}"
    );
    handle.abort();
    Ok(())
}
