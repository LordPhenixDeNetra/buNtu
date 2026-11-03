//! Hooks pré et post : points d'extension du pipeline.
//!
//! Aucun hook n'est bloquant par conception : ils sont asynchrones et doivent rendre la main
//! rapidement. Les hooks de flux reçoivent l'usage final via un événement de fin, jamais chaque
//! morceau.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use buntu_schema::{GatewayError, Request, Response, StopReason, Usage};

use crate::context::RequestContext;

/// Futur renvoyé par un hook ou un exécuteur.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, GatewayError>> + Send + 'a>>;

/// Événement de fin d'un flux, transmis aux hooks post.
#[derive(Debug, Clone)]
pub struct FinishEvent {
    pub usage: Usage,
    pub stop_reason: Option<StopReason>,
}

/// Résultat d'un hook pré : continuer ou répondre sans appeler le fournisseur.
#[derive(Debug)]
pub enum PreOutcome {
    Continue,
    Respond(Response),
}

/// Ce qu'un hook post observe : une réponse complète, la fin d'un flux, ou un échec.
#[derive(Clone, Copy)]
pub enum PostOutcome<'a> {
    Complete(&'a Response),
    StreamFinished(&'a FinishEvent),
    Failed(&'a GatewayError),
}

/// Hook exécuté avant l'appel : peut modifier la requête ou court-circuiter.
pub trait PreHook: Send + Sync {
    fn name(&self) -> &'static str;

    fn run<'a>(
        &'a self,
        context: &'a mut RequestContext,
        request: &'a mut Request,
    ) -> BoxFuture<'a, PreOutcome>;
}

/// Hook exécuté après l'appel : observe le résultat, ne modifie pas la réponse.
pub trait PostHook: Send + Sync {
    fn name(&self) -> &'static str;

    fn run<'a>(
        &'a self,
        context: &'a mut RequestContext,
        outcome: PostOutcome<'a>,
    ) -> BoxFuture<'a, ()>;
}

/// Hooks disponibles, indexés par leur nom déclaré.
#[derive(Default, Clone)]
pub struct HookRegistry {
    pre: HashMap<String, Arc<dyn PreHook>>,
    post: HashMap<String, Arc<dyn PostHook>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_pre(&mut self, hook: Arc<dyn PreHook>) {
        self.pre.insert(hook.name().to_owned(), hook);
    }

    pub fn register_post(&mut self, hook: Arc<dyn PostHook>) {
        self.post.insert(hook.name().to_owned(), hook);
    }

    pub(crate) fn pre_hook(&self, name: &str) -> Option<Arc<dyn PreHook>> {
        self.pre.get(name).cloned()
    }

    pub(crate) fn post_hook(&self, name: &str) -> Option<Arc<dyn PostHook>> {
        self.post.get(name).cloned()
    }
}
