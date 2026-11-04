//! Erreurs normalisées de la passerelle (T1.15).
//!
//! Chaque erreur possède un **code stable**, un **statut HTTP**, un **type** au format OpenAI
//! et une **source** exposée dans l'en-tête `x-buntu-error-source`. Les messages ne
//! contiennent jamais de clé, ni d'en-tête d'authentification, ni de corps de requête.

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
    #[error("all targets failed ({last}) after attempts: {attempts:?}")]
    AllTargetsFailed {
        /// Description de chaque tentative, dans l'ordre.
        attempts: Vec<String>,
        /// Dernière erreur rencontrée.
        last: Box<GatewayError>,
    },
    #[error("internal error: {message}")]
    Internal { message: String },
}

impl GatewayError {
    /// Code stable, utilisable par les clients (`error.code`).
    pub fn code(&self) -> &'static str {
        match self {
            GatewayError::ProviderRateLimited { .. } => "provider_rate_limited",
            GatewayError::ProviderUnavailable { .. } => "provider_unavailable",
            GatewayError::Timeout => "timeout",
            GatewayError::InvalidRequest { .. } => "invalid_request",
            GatewayError::Authentication => "authentication",
            GatewayError::BudgetExceeded => "budget_exceeded",
            GatewayError::AllTargetsFailed { .. } => "all_targets_failed",
            GatewayError::Internal { .. } => "internal_error",
        }
    }

    /// Statut HTTP associé.
    pub fn http_status(&self) -> u16 {
        match self {
            GatewayError::ProviderRateLimited { .. } => 429,
            GatewayError::ProviderUnavailable { .. } => 502,
            GatewayError::Timeout => 504,
            GatewayError::InvalidRequest { .. } => 400,
            GatewayError::Authentication => 401,
            GatewayError::BudgetExceeded => 402,
            GatewayError::AllTargetsFailed { .. } => 502,
            GatewayError::Internal { .. } => 500,
        }
    }

    /// Type d'erreur au format OpenAI (`error.type`).
    pub fn error_type(&self) -> &'static str {
        match self {
            GatewayError::ProviderRateLimited { .. } => "rate_limit_error",
            GatewayError::ProviderUnavailable { .. }
            | GatewayError::Timeout
            | GatewayError::AllTargetsFailed { .. } => "api_error",
            GatewayError::InvalidRequest { .. } => "invalid_request_error",
            GatewayError::Authentication => "authentication_error",
            GatewayError::BudgetExceeded => "insufficient_quota",
            GatewayError::Internal { .. } => "server_error",
        }
    }

    /// Origine de l'erreur, exposée dans `x-buntu-error-source`.
    pub fn source(&self) -> &'static str {
        match self {
            GatewayError::ProviderRateLimited { .. }
            | GatewayError::ProviderUnavailable { .. }
            | GatewayError::Timeout
            | GatewayError::AllTargetsFailed { .. } => "provider",
            GatewayError::InvalidRequest { .. } | GatewayError::Authentication => "client",
            GatewayError::BudgetExceeded => "budget",
            GatewayError::Internal { .. } => "gateway",
        }
    }

    /// Délai à indiquer au client (`retry-after`), s'il est connu.
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            GatewayError::ProviderRateLimited { retry_after } => *retry_after,
            _ => None,
        }
    }
}
