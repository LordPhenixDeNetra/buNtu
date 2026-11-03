//! Chaîne de repli : réessais sur une cible (T1.5), puis cible suivante.
//!
//! Aucun flux SSE n'est réessayé ici : la règle « après le premier octet, l'erreur se transmet
//! telle quelle » est portée par `retry::is_retryable`, et ce module ne traite que des réponses
//! complètes. Le chemin de flux (T1.13) devra appeler cette chaîne avant d'envoyer le moindre
//! octet, puis abandonner tout réessai.

use buntu_schema::{GatewayError, Request, Response};

use crate::context::RequestContext;
use crate::hooks::BoxFuture;
use crate::retry::{Backoff, ErrorClass, RetryBudget, classify, is_retryable};
use crate::routing::Target;

pub const HEADER_PROVIDER: &str = "x-buntu-provider";
pub const HEADER_MODEL: &str = "x-buntu-model";
pub const HEADER_ATTEMPTS: &str = "x-buntu-attempts";

/// Exécute une tentative contre une cible donnée (client fournisseur, simulateur…).
pub trait TargetExecutor: Send + Sync {
    fn execute<'a>(
        &'a self,
        target: &'a Target,
        context: &'a RequestContext,
        request: &'a Request,
    ) -> BoxFuture<'a, Response>;
}

/// Résultat d'une chaîne de repli, avec la provenance à exposer dans les en-têtes.
#[derive(Debug)]
pub struct RoutedResponse {
    pub response: Response,
    pub provider: String,
    pub model: String,
    pub attempts: u32,
}

impl RoutedResponse {
    /// En-têtes `x-buntu-*` à recopier dans la réponse HTTP (T1.13).
    pub fn headers(&self) -> [(&'static str, String); 3] {
        [
            (HEADER_PROVIDER, self.provider.clone()),
            (HEADER_MODEL, self.model.clone()),
            (HEADER_ATTEMPTS, self.attempts.to_string()),
        ]
    }
}

/// Orchestration : pour chaque cible, réessais tant que l'erreur est récupérable, puis cible
/// suivante ; la dernière erreur est renvoyée avec la liste des tentatives.
pub struct FallbackChain {
    max_attempts: u32,
    backoff: Backoff,
    budget: RetryBudget,
}

impl Default for FallbackChain {
    fn default() -> Self {
        Self::new(3, Backoff::default(), RetryBudget::default())
    }
}

impl FallbackChain {
    pub fn new(max_attempts: u32, backoff: Backoff, budget: RetryBudget) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
            backoff,
            budget,
        }
    }

    pub async fn execute(
        &self,
        targets: &[Target],
        executor: &dyn TargetExecutor,
        context: &RequestContext,
        request: &Request,
    ) -> Result<RoutedResponse, GatewayError> {
        self.budget.record_request();

        let mut attempts = 0_u32;
        let mut failures: Vec<String> = Vec::new();
        let mut last_error: Option<GatewayError> = None;

        for target in targets {
            let mut attempt = 0_u32;
            loop {
                attempt += 1;
                attempts += 1;
                match executor.execute(target, context, request).await {
                    Ok(response) => {
                        return Ok(RoutedResponse {
                            response,
                            provider: target.provider.clone(),
                            model: target.model.clone(),
                            attempts,
                        });
                    }
                    Err(error) => {
                        let retryable = classify(&error) == ErrorClass::Recoverable
                            && is_retryable(&error, false)
                            && attempt < self.max_attempts
                            && self.budget.try_retry();
                        if retryable {
                            tokio::time::sleep(self.backoff.delay(attempt - 1)).await;
                            continue;
                        }
                        failures.push(format!("{} ({} tentative(s))", target.label(), attempt));
                        last_error = Some(error);
                        break;
                    }
                }
            }
        }

        match last_error {
            Some(last) => Err(GatewayError::AllTargetsFailed {
                attempts: failures,
                last: Box::new(last),
            }),
            None => Err(GatewayError::Internal {
                message: "aucune cible à essayer".to_owned(),
            }),
        }
    }
}
