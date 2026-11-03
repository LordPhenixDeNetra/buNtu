//! Quatre timeouts de la requête amont : connexion, premier octet, inter-morceaux, total.
//!
//! Le piège de la fiche est explicite : un `timeout` posé sur toute la requête tuerait les
//! longs flux légitimes. On distingue donc :
//! - la **connexion** (établissement TCP/TLS), réglée par le client amont ;
//! - le **premier octet** (ou premier morceau de flux) : 60 s par défaut, relevable par route
//!   pour les modèles à réflexion ;
//! - l'**inter-morceaux** : chaque nouveau morceau doit arriver dans les 30 s ;
//! - le **total** : plafond absolu de 10 min, qui prime sur les autres.

use std::future::Future;
use std::time::{Duration, Instant};

use buntu_schema::{GatewayError, Request, Response};

use crate::context::RequestContext;
use crate::fallback::TargetExecutor;
use crate::hooks::BoxFuture;
use crate::routing::Target;

/// Les quatre délais d'une requête amont.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeouts {
    pub connect: Duration,
    pub first_byte: Duration,
    pub inter_chunk: Duration,
    pub total: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(5),
            first_byte: Duration::from_secs(60),
            inter_chunk: Duration::from_secs(30),
            total: Duration::from_secs(600),
        }
    }
}

impl Timeouts {
    /// Applique des surcharges (champs `None` = valeur héritée).
    pub fn apply(
        mut self,
        connect: Option<Duration>,
        first_byte: Option<Duration>,
        inter_chunk: Option<Duration>,
        total: Option<Duration>,
    ) -> Self {
        if let Some(value) = connect {
            self.connect = value;
        }
        if let Some(value) = first_byte {
            self.first_byte = value;
        }
        if let Some(value) = inter_chunk {
            self.inter_chunk = value;
        }
        if let Some(value) = total {
            self.total = value;
        }
        self
    }
}

/// Applique un timeout à un futur, en convertissant l'expiration en `GatewayError::Timeout`.
///
/// Le futur est **abandonné** (donc la connexion sous-jacente fermée) dès l'expiration : c'est
/// ce que vérifient les tests avec un garde de destruction.
pub async fn with_timeout<F, T>(duration: Duration, future: F) -> Result<T, GatewayError>
where
    F: Future<Output = Result<T, GatewayError>>,
{
    match tokio::time::timeout(duration, future).await {
        Ok(result) => result,
        Err(_elapsed) => Err(GatewayError::Timeout),
    }
}

/// Enveloppe un exécuteur de cible avec le premier octet et le total.
///
/// Le timeout de connexion reste la responsabilité du client amont (ADR-0004) : il s'applique à
/// l'établissement de la connexion, pas à l'appel complet, et n'a donc pas sa place ici.
pub struct TimedExecutor<'a, E: TargetExecutor + ?Sized> {
    inner: &'a E,
    timeouts: Timeouts,
}

impl<'a, E: TargetExecutor + ?Sized> TimedExecutor<'a, E> {
    pub fn new(inner: &'a E, timeouts: Timeouts) -> Self {
        Self { inner, timeouts }
    }

    pub fn timeouts(&self) -> Timeouts {
        self.timeouts
    }
}

impl<E: TargetExecutor + ?Sized> TargetExecutor for TimedExecutor<'_, E> {
    fn execute<'a>(
        &'a self,
        target: &'a Target,
        context: &'a RequestContext,
        request: &'a Request,
    ) -> BoxFuture<'a, Response> {
        let timeouts = self.timeouts;
        Box::pin(async move {
            with_timeout(timeouts.total, async {
                with_timeout(
                    timeouts.first_byte,
                    self.inner.execute(target, context, request),
                )
                .await
            })
            .await
        })
    }
}

/// Suivi des timeouts d'un flux : premier morceau, puis inter-morceaux, sous plafond total.
#[derive(Debug, Clone)]
pub struct StreamTimeouts {
    timeouts: Timeouts,
    started: Instant,
    first_received: bool,
}

impl StreamTimeouts {
    pub fn new(timeouts: Timeouts, started: Instant) -> Self {
        Self {
            timeouts,
            started,
            first_received: false,
        }
    }

    /// Délai maximal pour le prochain morceau, ou `Timeout` si le plafond total est atteint.
    pub fn next_budget(&self) -> Result<Duration, GatewayError> {
        let remaining = self.remaining_total();
        if remaining.is_zero() {
            return Err(GatewayError::Timeout);
        }
        let nominal = if self.first_received {
            self.timeouts.inter_chunk
        } else {
            self.timeouts.first_byte
        };
        Ok(nominal.min(remaining))
    }

    /// À appeler après la réception d'un morceau.
    pub fn received_chunk(&mut self) {
        self.first_received = true;
    }

    pub fn remaining_total(&self) -> Duration {
        self.timeouts.total.saturating_sub(self.started.elapsed())
    }
}
