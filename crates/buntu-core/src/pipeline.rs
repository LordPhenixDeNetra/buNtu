//! Pipeline de requête : un seul endroit où la requête traverse les hooks et l'appel.

use std::sync::Arc;

use buntu_schema::{GatewayError, Request, Response};

use crate::config::CompiledConfig;
use crate::context::RequestContext;
use crate::hooks::{
    BoxFuture, FinishEvent, HookRegistry, PostHook, PostOutcome, PreHook, PreOutcome,
};

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("hook pré inconnu : `{name}`")]
    UnknownPreHook { name: String },
    #[error("hook post inconnu : `{name}`")]
    UnknownPostHook { name: String },
}

/// Exécute l'appel effectif (routage, fournisseur, mock) : injecté par l'appelant.
pub trait RequestExecutor: Send + Sync {
    fn execute<'a>(
        &'a self,
        context: &'a RequestContext,
        request: &'a Request,
    ) -> BoxFuture<'a, Response>;
}

/// Chaîne ordonnée de hooks pré et post.
pub struct Pipeline {
    pre: Vec<Arc<dyn PreHook>>,
    post: Vec<Arc<dyn PostHook>>,
}

impl Pipeline {
    /// Pipeline sans hooks (chemin minimal, utilisé par les tests et les bancs).
    pub fn empty() -> Self {
        Self {
            pre: Vec::new(),
            post: Vec::new(),
        }
    }

    /// Construit la chaîne dans l'ordre déclaré par la configuration compilée.
    pub fn build(config: &CompiledConfig, registry: &HookRegistry) -> Result<Self, PipelineError> {
        let mut pre = Vec::with_capacity(config.hooks.pre.len());
        for name in &config.hooks.pre {
            let hook = registry
                .pre_hook(name)
                .ok_or_else(|| PipelineError::UnknownPreHook { name: name.clone() })?;
            pre.push(hook);
        }
        let mut post = Vec::with_capacity(config.hooks.post.len());
        for name in &config.hooks.post {
            let hook = registry
                .post_hook(name)
                .ok_or_else(|| PipelineError::UnknownPostHook { name: name.clone() })?;
            post.push(hook);
        }
        Ok(Self { pre, post })
    }

    /// Traite une requête complète : hooks pré, appel, hooks post.
    pub async fn handle(
        &self,
        context: &mut RequestContext,
        request: &mut Request,
        executor: &dyn RequestExecutor,
    ) -> Result<Response, GatewayError> {
        for hook in &self.pre {
            if let PreOutcome::Respond(response) = hook.run(context, request).await? {
                self.run_post(context, PostOutcome::Complete(&response))
                    .await?;
                return Ok(response);
            }
        }

        match executor.execute(context, request).await {
            Ok(response) => {
                self.run_post(context, PostOutcome::Complete(&response))
                    .await?;
                Ok(response)
            }
            Err(error) => {
                self.run_post(context, PostOutcome::Failed(&error)).await?;
                Err(error)
            }
        }
    }

    /// Signale la fin d'un flux aux hooks post (usage final, jamais chaque morceau).
    pub async fn finish_stream(
        &self,
        context: &mut RequestContext,
        event: &FinishEvent,
    ) -> Result<(), GatewayError> {
        self.run_post(context, PostOutcome::StreamFinished(event))
            .await
    }

    async fn run_post(
        &self,
        context: &mut RequestContext,
        outcome: PostOutcome<'_>,
    ) -> Result<(), GatewayError> {
        for hook in &self.post {
            hook.run(context, outcome).await?;
        }
        Ok(())
    }
}
