//! Tests du pipeline et de la chaîne de hooks.

use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use buntu_core::config::{CompiledConfig, MapEnvironment};
use buntu_core::context::RequestContext;
use buntu_core::hooks::{
    BoxFuture, FinishEvent, HookRegistry, PostHook, PostOutcome, PreHook, PreOutcome,
};
use buntu_core::pipeline::{Pipeline, PipelineError, RequestExecutor};
use buntu_schema::{GatewayError, Message, Request, Response, Role, StopReason, Usage};
use serde_json::Map;

const CONFIG_WITH_HOOKS: &str = r#"
[server]
listen = "127.0.0.1:8080"

[hooks]
pre = ["a", "b"]
post = ["c", "d"]
"#;

fn sample_request() -> Request {
    Request {
        model: "modele".to_owned(),
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

fn remember(log: &Arc<Mutex<Vec<String>>>, entry: &str) -> Result<(), GatewayError> {
    let mut guard = log.lock().map_err(|_| GatewayError::Internal {
        message: "verrou empoisonné".to_owned(),
    })?;
    guard.push(entry.to_owned());
    Ok(())
}

fn entries(log: &Arc<Mutex<Vec<String>>>) -> Result<Vec<String>, GatewayError> {
    let guard = log.lock().map_err(|_| GatewayError::Internal {
        message: "verrou empoisonné".to_owned(),
    })?;
    Ok(guard.clone())
}

struct RecordingPre {
    name: &'static str,
    log: Arc<Mutex<Vec<String>>>,
}

impl PreHook for RecordingPre {
    fn name(&self) -> &'static str {
        self.name
    }

    fn run<'a>(
        &'a self,
        _context: &'a mut RequestContext,
        _request: &'a mut Request,
    ) -> BoxFuture<'a, PreOutcome> {
        Box::pin(async move {
            remember(&self.log, self.name)?;
            Ok(PreOutcome::Continue)
        })
    }
}

struct ShortCircuitPre {
    log: Arc<Mutex<Vec<String>>>,
}

impl PreHook for ShortCircuitPre {
    fn name(&self) -> &'static str {
        "short"
    }

    fn run<'a>(
        &'a self,
        _context: &'a mut RequestContext,
        _request: &'a mut Request,
    ) -> BoxFuture<'a, PreOutcome> {
        Box::pin(async move {
            remember(&self.log, self.name())?;
            Ok(PreOutcome::Respond(sample_response("court-circuit")))
        })
    }
}

struct FailingPre;

impl PreHook for FailingPre {
    fn name(&self) -> &'static str {
        "fail"
    }

    fn run<'a>(
        &'a self,
        _context: &'a mut RequestContext,
        _request: &'a mut Request,
    ) -> BoxFuture<'a, PreOutcome> {
        Box::pin(async move {
            Err(GatewayError::Internal {
                message: "hook pré en échec".to_owned(),
            })
        })
    }
}

struct RecordingPost {
    name: &'static str,
    log: Arc<Mutex<Vec<String>>>,
}

impl PostHook for RecordingPost {
    fn name(&self) -> &'static str {
        self.name
    }

    fn run<'a>(
        &'a self,
        _context: &'a mut RequestContext,
        _outcome: PostOutcome<'a>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            remember(&self.log, self.name)?;
            Ok(())
        })
    }
}

struct UsagePost {
    log: Arc<Mutex<Vec<String>>>,
}

impl PostHook for UsagePost {
    fn name(&self) -> &'static str {
        "usage"
    }

    fn run<'a>(
        &'a self,
        _context: &'a mut RequestContext,
        outcome: PostOutcome<'a>,
    ) -> BoxFuture<'a, ()> {
        let label = match outcome {
            PostOutcome::Complete(_) => "complete",
            PostOutcome::StreamFinished(event) => {
                if event.usage.output_tokens == 7 {
                    "flux:7"
                } else {
                    "flux:autre"
                }
            }
            PostOutcome::Failed(_) => "failed",
        };
        Box::pin(async move {
            remember(&self.log, label)?;
            Ok(())
        })
    }
}

struct FixedExecutor {
    response: Response,
    called: Arc<AtomicBool>,
}

impl RequestExecutor for FixedExecutor {
    fn execute<'a>(
        &'a self,
        _context: &'a RequestContext,
        _request: &'a Request,
    ) -> BoxFuture<'a, Response> {
        let called = Arc::clone(&self.called);
        let response = self.response.clone();
        Box::pin(async move {
            called.store(true, Ordering::SeqCst);
            Ok(response)
        })
    }
}

struct FailingExecutor;

impl RequestExecutor for FailingExecutor {
    fn execute<'a>(
        &'a self,
        _context: &'a RequestContext,
        _request: &'a Request,
    ) -> BoxFuture<'a, Response> {
        Box::pin(async move { Err(GatewayError::Timeout) })
    }
}

fn compile(content: &str) -> Result<CompiledConfig, Box<dyn Error>> {
    Ok(buntu_core::config::from_str(
        content,
        &MapEnvironment::default(),
    )?)
}

fn recorder_pre(name: &'static str, log: &Arc<Mutex<Vec<String>>>) -> Arc<dyn PreHook> {
    Arc::new(RecordingPre {
        name,
        log: Arc::clone(log),
    })
}

fn recorder_post(name: &'static str, log: &Arc<Mutex<Vec<String>>>) -> Arc<dyn PostHook> {
    Arc::new(RecordingPost {
        name,
        log: Arc::clone(log),
    })
}

#[tokio::test]
async fn hooks_run_in_declared_order() -> Result<(), Box<dyn Error>> {
    let config = compile(CONFIG_WITH_HOOKS)?;
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut registry = HookRegistry::new();
    registry.register_pre(recorder_pre("a", &log));
    registry.register_pre(recorder_pre("b", &log));
    registry.register_post(recorder_post("c", &log));
    registry.register_post(recorder_post("d", &log));
    let pipeline = Pipeline::build(&config, &registry)?;

    let executor = FixedExecutor {
        response: sample_response("ok"),
        called: Arc::new(AtomicBool::new(false)),
    };
    let mut context = RequestContext::new("ordre");
    let mut request = sample_request();
    let _response = pipeline
        .handle(&mut context, &mut request, &executor)
        .await?;

    assert_eq!(entries(&log).map_err(|_| "log")?.join(","), "a,b,c,d");
    Ok(())
}

#[tokio::test]
async fn pre_hook_can_short_circuit_without_calling_the_executor() -> Result<(), Box<dyn Error>> {
    let config = compile(
        "[server]\nlisten = \"127.0.0.1:8080\"\n\n[hooks]\npre = [\"short\"]\npost = [\"c\"]\n",
    )?;
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut registry = HookRegistry::new();
    registry.register_pre(Arc::new(ShortCircuitPre {
        log: Arc::clone(&log),
    }));
    registry.register_post(recorder_post("c", &log));
    let pipeline = Pipeline::build(&config, &registry)?;

    let called = Arc::new(AtomicBool::new(false));
    let executor = FixedExecutor {
        response: sample_response("jamais-appelé"),
        called: Arc::clone(&called),
    };
    let mut context = RequestContext::new("court-circuit");
    let mut request = sample_request();
    let response = pipeline
        .handle(&mut context, &mut request, &executor)
        .await?;

    assert_eq!(response.id, "court-circuit");
    assert!(!called.load(Ordering::SeqCst), "l'exécuteur a été appelé");
    assert_eq!(entries(&log).map_err(|_| "log")?.join(","), "short,c");
    Ok(())
}

#[tokio::test]
async fn error_in_pre_hook_is_a_clean_error() -> Result<(), Box<dyn Error>> {
    let config = compile("[server]\nlisten = \"127.0.0.1:8080\"\n\n[hooks]\npre = [\"fail\"]\n")?;
    let mut registry = HookRegistry::new();
    registry.register_pre(Arc::new(FailingPre));
    let pipeline = Pipeline::build(&config, &registry)?;

    let called = Arc::new(AtomicBool::new(false));
    let executor = FixedExecutor {
        response: sample_response("jamais"),
        called: Arc::clone(&called),
    };
    let mut context = RequestContext::new("échec");
    let mut request = sample_request();
    let result = pipeline.handle(&mut context, &mut request, &executor).await;

    assert!(result.is_err());
    assert!(!called.load(Ordering::SeqCst));
    Ok(())
}

#[tokio::test]
async fn executor_error_still_runs_post_hooks() -> Result<(), Box<dyn Error>> {
    let config = compile("[server]\nlisten = \"127.0.0.1:8080\"\n\n[hooks]\npost = [\"c\"]\n")?;
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut registry = HookRegistry::new();
    registry.register_post(recorder_post("c", &log));
    let pipeline = Pipeline::build(&config, &registry)?;

    let mut context = RequestContext::new("échec-appel");
    let mut request = sample_request();
    let result = pipeline
        .handle(&mut context, &mut request, &FailingExecutor)
        .await;

    assert!(matches!(result, Err(GatewayError::Timeout)));
    assert_eq!(entries(&log).map_err(|_| "log")?.join(","), "c");
    Ok(())
}

#[tokio::test]
async fn error_in_post_hook_is_a_clean_error() -> Result<(), Box<dyn Error>> {
    struct FailingPost;
    impl PostHook for FailingPost {
        fn name(&self) -> &'static str {
            "post-fail"
        }

        fn run<'a>(
            &'a self,
            _context: &'a mut RequestContext,
            _outcome: PostOutcome<'a>,
        ) -> BoxFuture<'a, ()> {
            Box::pin(async move {
                Err(GatewayError::Internal {
                    message: "hook post en échec".to_owned(),
                })
            })
        }
    }

    let config =
        compile("[server]\nlisten = \"127.0.0.1:8080\"\n\n[hooks]\npost = [\"post-fail\"]\n")?;
    let mut registry = HookRegistry::new();
    registry.register_post(Arc::new(FailingPost));
    let pipeline = Pipeline::build(&config, &registry)?;

    let executor = FixedExecutor {
        response: sample_response("ok"),
        called: Arc::new(AtomicBool::new(false)),
    };
    let mut context = RequestContext::new("post");
    let mut request = sample_request();
    assert!(
        pipeline
            .handle(&mut context, &mut request, &executor)
            .await
            .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn stream_finish_reports_the_final_usage_once() -> Result<(), Box<dyn Error>> {
    let config = compile("[server]\nlisten = \"127.0.0.1:8080\"\n\n[hooks]\npost = [\"usage\"]\n")?;
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut registry = HookRegistry::new();
    registry.register_post(Arc::new(UsagePost {
        log: Arc::clone(&log),
    }));
    let pipeline = Pipeline::build(&config, &registry)?;

    let mut context = RequestContext::new("flux");
    let event = FinishEvent {
        usage: Usage {
            input_tokens: 3,
            output_tokens: 7,
            cached_input_tokens: 0,
            reasoning_tokens: 0,
        },
        stop_reason: Some(StopReason::EndTurn),
    };
    pipeline.finish_stream(&mut context, &event).await?;
    assert_eq!(entries(&log).map_err(|_| "log")?.join(","), "flux:7");
    Ok(())
}

#[test]
fn unknown_hook_is_rejected_when_building() -> Result<(), Box<dyn Error>> {
    let config = compile("[server]\nlisten = \"127.0.0.1:8080\"\n\n[hooks]\npre = [\"absent\"]\n")?;
    let registry = HookRegistry::new();
    match Pipeline::build(&config, &registry) {
        Ok(_) => panic!("une erreur était attendue"),
        Err(PipelineError::UnknownPreHook { name }) => assert_eq!(name, "absent"),
        Err(other) => panic!("erreur inattendue : {other:?}"),
    }
    Ok(())
}

#[test]
fn extensions_round_trip_a_typed_value() {
    let mut context = RequestContext::new("extensions");
    assert!(context.extensions.is_empty());
    context.extensions.insert(String::from("valeur"));
    assert_eq!(
        context.extensions.get::<String>().map(String::as_str),
        Some("valeur")
    );
    assert_eq!(context.extensions.len(), 1);
    assert_eq!(
        context.extensions.remove::<String>(),
        Some("valeur".to_owned())
    );
    assert!(context.extensions.is_empty());
}

#[test]
fn empty_chain_is_under_one_microsecond() -> Result<(), Box<dyn Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let pipeline = Pipeline::empty();
    let executor = FixedExecutor {
        response: sample_response("bench"),
        called: Arc::new(AtomicBool::new(false)),
    };

    runtime.block_on(async {
        let mut context = RequestContext::new("banc");
        let mut request = sample_request();
        for _ in 0..500 {
            let _ = pipeline
                .handle(&mut context, &mut request, &executor)
                .await?;
        }

        let iterations = 20_000_u32;
        let started = Instant::now();
        for _ in 0..iterations {
            let _ = pipeline
                .handle(&mut context, &mut request, &executor)
                .await?;
        }
        let elapsed = started.elapsed();
        let per_iteration = elapsed.as_nanos() as f64 / f64::from(iterations);
        println!("chaîne vide : {per_iteration:.0} ns par requête");
        let limit = if cfg!(debug_assertions) {
            10_000.0
        } else {
            1_000.0
        };
        assert!(
            per_iteration < limit,
            "{per_iteration:.0} ns par requête, limite {limit:.0} ns"
        );
        Ok::<(), Box<dyn Error>>(())
    })?;
    Ok(())
}
