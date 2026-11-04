//! Métriques Prometheus (T1.19).
//!
//! Seuls des labels à **faible cardinalité** sont utilisés (fournisseur, statut, sens) : jamais
//! d'identifiant de requête ni de clé virtuelle. Les histogrammes ont leurs propres seaux : µs
//! pour le surcoût interne, secondes pour la durée totale.

use std::sync::OnceLock;
use std::time::Duration;

use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle, PrometheusRecorder};

/// Seaux du surcoût interne, en microsecondes.
pub const OVERHEAD_BUCKETS_MICROS: &[f64] = &[
    5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1_000.0, 2_500.0, 5_000.0, 10_000.0,
];

/// Seaux de la durée totale, en secondes.
pub const DURATION_BUCKETS_SECONDS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0,
];

/// Exportateur Prometheus prêt à installer, avec son handle de rendu.
pub struct Metrics {
    recorder: PrometheusRecorder,
    handle: PrometheusHandle,
}

impl Metrics {
    pub fn build() -> Result<Self, String> {
        let builder = PrometheusBuilder::new()
            .set_buckets_for_metric(
                metrics_exporter_prometheus::Matcher::Full(
                    "buntu_request_overhead_microseconds".to_owned(),
                ),
                OVERHEAD_BUCKETS_MICROS,
            )
            .map_err(|error| error.to_string())?
            .set_buckets_for_metric(
                metrics_exporter_prometheus::Matcher::Full(
                    "buntu_request_duration_seconds".to_owned(),
                ),
                DURATION_BUCKETS_SECONDS,
            )
            .map_err(|error| error.to_string())?;
        let recorder = builder.build_recorder();
        let handle = recorder.handle();
        Ok(Self { recorder, handle })
    }

    pub fn handle(&self) -> &PrometheusHandle {
        &self.handle
    }

    /// Recorder pour un rendu isolé (`metrics::with_local_recorder`) dans les tests.
    pub fn recorder(&self) -> &PrometheusRecorder {
        &self.recorder
    }

    /// Installe l'exportateur global (une seule fois par processus) et rend son handle.
    pub fn install(self) -> Result<PrometheusHandle, String> {
        let handle = self.handle.clone();
        metrics::set_global_recorder(self.recorder).map_err(|error| error.to_string())?;
        Ok(handle)
    }
}

static GLOBAL_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Installe l'exportateur global au démarrage du binaire ; idempotent.
pub fn install_global() -> Result<&'static PrometheusHandle, String> {
    if let Some(handle) = GLOBAL_HANDLE.get() {
        return Ok(handle);
    }
    let handle = Metrics::build()?.install()?;
    let _ = GLOBAL_HANDLE.set(handle);
    GLOBAL_HANDLE
        .get()
        .ok_or_else(|| "exportateur Prometheus non installé".to_owned())
}

/// Rend l'exposition Prometheus si l'exportateur est installé.
pub fn render_global() -> Option<String> {
    GLOBAL_HANDLE.get().map(PrometheusHandle::render)
}

/// Enregistre une requête terminée (fournisseur, statut, durée, surcoût interne).
pub fn record_request(provider: &str, status: u16, total: Duration, overhead: Duration) {
    metrics::counter!(
        "buntu_requests_total",
        "provider" => provider.to_owned(),
        "status" => status.to_string(),
    )
    .increment(1);
    metrics::histogram!("buntu_request_duration_seconds").record(total.as_secs_f64());
    metrics::histogram!("buntu_request_overhead_microseconds")
        .record(overhead.as_secs_f64() * 1_000_000.0);
}

/// Enregistre la durée d'un appel amont.
pub fn record_upstream(provider: &str, elapsed: Duration) {
    metrics::histogram!(
        "buntu_upstream_duration_seconds",
        "provider" => provider.to_owned(),
    )
    .record(elapsed.as_secs_f64());
}

/// Enregistre les jetons consommés (`direction` = `input` ou `output`).
pub fn record_tokens(direction: &str, tokens: u64) {
    metrics::counter!(
        "buntu_tokens_total",
        "direction" => direction.to_owned(),
    )
    .increment(tokens);
}

/// Enregistre un repli de fournisseur (nombre de tentatives supplémentaires).
pub fn record_fallback(provider: &str) {
    metrics::counter!(
        "buntu_fallback_total",
        "provider" => provider.to_owned(),
    )
    .increment(1);
}
