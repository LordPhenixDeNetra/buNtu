//! Erreurs normalisées de la passerelle.
//!
//! Les codes stables et les statuts HTTP seront ajoutés par T1.15 ; ce type pose le
//! vocabulaire commun aux crates.

use std::time::Duration;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GatewayError {
    #[error("provider rate limited")]
    ProviderRateLimited { retry_after: Option<Duration> },
    #[error("provider unavailable: {message}")]
    ProviderUnavailable { message: String },
    #[error("upstream timeout")]
    Timeout,
    #[error("invalid request: {message}")]
    InvalidRequest { message: String },
    #[error("authentication failed")]
    Authentication,
    #[error("budget exceeded")]
    BudgetExceeded,
    #[error("all targets failed")]
    AllTargetsFailed { attempts: Vec<String> },
    #[error("internal error: {message}")]
    Internal { message: String },
}
