//! Limitation des tentatives de connexion admin (T1.20).
//!
//! Dix échecs consécutifs bloquent le compte pour une durée temporaire ; un succès remet le
//! compteur à zéro.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Nombre d'échecs avant blocage.
pub const MAX_ATTEMPTS: u32 = 10;
/// Durée du blocage temporaire.
pub const BLOCK_DURATION: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, Copy)]
struct Attempts {
    failures: u32,
    blocked_until: Option<Instant>,
}

/// Compteur de tentatives par compte.
#[derive(Debug, Default)]
pub struct LoginLimiter {
    inner: Mutex<HashMap<String, Attempts>>,
}

/// Blocage temporaire : indique le temps restant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoginBlocked {
    pub retry_after: Duration,
}

impl LoginLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Vérifie si le compte peut tenter une connexion.
    pub fn check(&self, account: &str) -> Result<(), LoginBlocked> {
        let attempts = self.lock().get(account).copied().unwrap_or(Attempts {
            failures: 0,
            blocked_until: None,
        });
        match attempts.blocked_until {
            Some(until) if until > Instant::now() => Err(LoginBlocked {
                retry_after: until.saturating_duration_since(Instant::now()),
            }),
            _ => Ok(()),
        }
    }

    /// Enregistre un échec ; le dixième échec bloque le compte.
    pub fn record_failure(&self, account: &str) -> Result<(), LoginBlocked> {
        let now = Instant::now();
        let mut map = self.lock();
        let attempts = map.entry(account.to_owned()).or_insert(Attempts {
            failures: 0,
            blocked_until: None,
        });
        if let Some(until) = attempts.blocked_until
            && until > now
        {
            return Err(LoginBlocked {
                retry_after: until.saturating_duration_since(now),
            });
        }
        attempts.failures += 1;
        if attempts.failures >= MAX_ATTEMPTS {
            attempts.failures = 0;
            attempts.blocked_until = Some(now + BLOCK_DURATION);
            return Err(LoginBlocked {
                retry_after: BLOCK_DURATION,
            });
        }
        Ok(())
    }

    /// Remet le compteur à zéro après une connexion réussie.
    pub fn record_success(&self, account: &str) {
        self.lock().remove(account);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Attempts>> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
