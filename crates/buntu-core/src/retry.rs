//! Politique de réessais : classification, backoff avec gigue complète, budget global.
//!
//! Ce module ne dort pas et ne fait aucun appel : il décide *si* et *quand* réessayer, et
//! laisse l'attente au chef d'orchestre (T1.6). La règle du flux est explicite : après le
//! premier octet envoyé au client, aucune erreur n'est réessayée.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use buntu_schema::GatewayError;

/// Classe d'une erreur vis-à-vis du réessai.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    /// Réessayer peut réussir (429, 5xx, timeout, connexion réinitialisée).
    Recoverable,
    /// Réessayer est inutile ou dangereux (requête invalide, authentification, budget).
    NonRecoverable,
}

/// Classe un statut HTTP amont, selon la liste de la fiche T1.5.
///
/// Récupérables : 429, 500, 502, 503, 504. Non récupérables : 400, 401, 403, 404, 422.
/// Les autres statuts sont non récupérables par défaut : mieux vaut ne pas insister sur une
/// réponse inattendue.
pub fn classify_status(status: u16) -> ErrorClass {
    match status {
        429 | 500 | 502 | 503 | 504 => ErrorClass::Recoverable,
        _ => ErrorClass::NonRecoverable,
    }
}

/// Classe une erreur normalisée de la passerelle.
pub fn classify(error: &GatewayError) -> ErrorClass {
    match error {
        GatewayError::ProviderRateLimited { .. }
        | GatewayError::ProviderUnavailable { .. }
        | GatewayError::Timeout => ErrorClass::Recoverable,
        GatewayError::InvalidRequest { .. }
        | GatewayError::Authentication
        | GatewayError::BudgetExceeded
        | GatewayError::AllTargetsFailed { .. }
        | GatewayError::Internal { .. } => ErrorClass::NonRecoverable,
    }
}

/// Décide si une erreur peut être réessayée, en tenant compte du flux.
///
/// Piège crucial de la fiche : un flux déjà commencé côté client ne doit **jamais** être
/// réessayé, sinon le client recevrait deux débuts de réponse. Après le premier octet, l'erreur
/// se transmet telle quelle.
pub fn is_retryable(error: &GatewayError, stream_started: bool) -> bool {
    !stream_started && classify(error) == ErrorClass::Recoverable
}

/// Backoff exponentiel avec **gigue complète** : le délai est tiré uniformément dans
/// `[0, min(base × 2^tentative, plafond)]`.
#[derive(Debug, Clone)]
pub struct Backoff {
    base: Duration,
    cap: Duration,
}

impl Default for Backoff {
    fn default() -> Self {
        Self {
            base: Duration::from_millis(100),
            cap: Duration::from_secs(8),
        }
    }
}

impl Backoff {
    pub fn new(base: Duration, cap: Duration) -> Self {
        Self { base, cap }
    }

    /// Délai d'attente avant la tentative `attempt` (0 = premier réessai).
    pub fn delay(&self, attempt: u32) -> Duration {
        let exponential = self.base.saturating_mul(1_u32 << attempt.min(31));
        let bounded = exponential.min(self.cap);
        let max_millis = bounded.as_millis().min(u128::from(u64::MAX)) as u64;
        Duration::from_millis(fastrand::u64(0..=max_millis))
    }
}

/// Budget global de réessais : au plus 20 % des requêtes, sur n'importe quelle fenêtre.
///
/// Implémentation par seau à jetons sans verrou : chaque requête ajoute 0,2 jeton (en
/// millièmes), chaque réessai en consomme un ; le seau est plafonné pour borner les rafales.
/// Sur la durée, le taux de réessais ne peut pas dépasser le ratio demandé — c'est la
/// protection contre l'effet boule de neige.
pub struct RetryBudget {
    tokens: AtomicU64,
    capacity: u64,
    tokens_per_request: u64,
}

impl Default for RetryBudget {
    fn default() -> Self {
        Self::new(20, 20)
    }
}

impl RetryBudget {
    /// `ratio_percent` : pourcentage maximal de réessais par requête (20 = 20 %).
    /// `capacity_tokens` : taille du seau, donc la rafale initiale maximale.
    pub fn new(ratio_percent: u32, capacity_tokens: u32) -> Self {
        let tokens_per_request = u64::from(ratio_percent) * 10;
        let capacity = u64::from(capacity_tokens) * 1000;
        Self {
            tokens: AtomicU64::new(capacity),
            capacity,
            tokens_per_request,
        }
    }

    /// Enregistre une requête : ajoute sa part de budget, sans dépasser la capacité.
    pub fn record_request(&self) {
        let mut current = self.tokens.load(Ordering::Relaxed);
        loop {
            let next = current
                .saturating_add(self.tokens_per_request)
                .min(self.capacity);
            match self.tokens.compare_exchange_weak(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return,
                Err(observed) => current = observed,
            }
        }
    }

    /// Consomme un jeton de réessai ; `false` si le budget est épuisé.
    pub fn try_retry(&self) -> bool {
        let mut current = self.tokens.load(Ordering::Relaxed);
        loop {
            if current < 1000 {
                return false;
            }
            match self.tokens.compare_exchange_weak(
                current,
                current - 1000,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(observed) => current = observed,
            }
        }
    }

    /// Jetons entiers disponibles (pour l'observabilité et les tests).
    pub fn available(&self) -> u64 {
        self.tokens.load(Ordering::Relaxed) / 1000
    }
}
