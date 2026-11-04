//! Preuves de T1.19 : métriques exposées, seaux dédiés, aucun label à forte cardinalité.

use std::time::Duration;

use buntu_observe::metrics::{DURATION_BUCKETS_SECONDS, Metrics, OVERHEAD_BUCKETS_MICROS};

fn build() -> Metrics {
    match Metrics::build() {
        Ok(metrics) => metrics,
        Err(error) => panic!("exportateur Prometheus non constructible : {error}"),
    }
}

#[test]
fn exposition_contains_the_required_metrics_without_high_cardinality_labels() {
    let local = build();
    let handle = local.handle().clone();
    metrics::with_local_recorder(local.recorder(), || {
        buntu_observe::metrics::record_request(
            "mock",
            200,
            Duration::from_millis(42),
            Duration::from_micros(7),
        );
        buntu_observe::metrics::record_upstream("mock", Duration::from_millis(40));
        buntu_observe::metrics::record_tokens("input", 12);
        buntu_observe::metrics::record_fallback("mock");
    });
    let rendered = handle.render();
    for name in [
        "buntu_requests_total",
        "buntu_request_duration_seconds",
        "buntu_request_overhead_microseconds",
        "buntu_upstream_duration_seconds",
        "buntu_tokens_total",
        "buntu_fallback_total",
    ] {
        assert!(
            rendered.contains(name),
            "métrique absente : {name}\n{rendered}"
        );
    }
    assert!(rendered.contains("provider=\"mock\""));
    assert!(rendered.contains("status=\"200\""));
    assert!(!rendered.contains("request_id"));
    assert!(!rendered.contains("virtual_key"));
}

#[test]
fn buckets_are_defined_for_microseconds_and_seconds() {
    assert_eq!(OVERHEAD_BUCKETS_MICROS.first(), Some(&5.0));
    assert!(OVERHEAD_BUCKETS_MICROS.len() >= 10);
    assert_eq!(DURATION_BUCKETS_SECONDS.first(), Some(&0.005));
    assert!(DURATION_BUCKETS_SECONDS.len() >= 10);
}

#[test]
fn global_installation_is_idempotent() {
    let first = buntu_observe::metrics::install_global();
    assert!(first.is_ok(), "installation initiale : {first:?}");
    let second = buntu_observe::metrics::install_global();
    assert!(second.is_ok(), "installation idempotente : {second:?}");
    assert!(buntu_observe::metrics::render_global().is_some());
}
