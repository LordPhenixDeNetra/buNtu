//! Résolution d'une requête vers une liste ordonnée de cibles.
//!
//! `provider/model` désigne une cible directe ; un alias est cherché dans les routes de la
//! configuration compilée, dans l'ordre déclaré. Le filtrage par capacités est branché ici :
//! tant que le catalogue de modèles (T2.6) n'est pas disponible, les cibles sont « inconnues »
//! (`None`) et ne sont jamais éliminées — le filtre est prêt, pas silencieusement ignoré.

use buntu_schema::{ContentPart, GatewayError, Request};

pub use buntu_schema::Capabilities;

use crate::config::CompiledConfig;

/// Une cible de routage : un fournisseur et un modèle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub provider: String,
    pub model: String,
    /// Capacités connues du modèle ; `None` tant que le catalogue (T2.6) n'est pas branché.
    pub capabilities: Option<Capabilities>,
}

impl Target {
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
            capabilities: None,
        }
    }

    pub fn label(&self) -> String {
        format!("{}/{}", self.provider, self.model)
    }
}

/// Capacités exigées par une requête (outils et vision, d'après le contenu canonique).
pub fn required_capabilities(request: &Request) -> Capabilities {
    let mut parts = request
        .messages
        .iter()
        .flat_map(|message| message.content.iter());
    let mut required = Capabilities {
        tools: !request.tools.is_empty(),
        vision: false,
    };
    for part in parts.by_ref() {
        match part {
            ContentPart::ToolCall(_) | ContentPart::ToolResult(_) => required.tools = true,
            ContentPart::Image { .. } => required.vision = true,
            _ => {}
        }
    }
    required
}

/// Résout `requested` en cibles ordonnées, puis filtre par capacités.
pub fn resolve(
    config: &CompiledConfig,
    requested: &str,
    required: Capabilities,
) -> Result<Vec<Target>, GatewayError> {
    let ordered = ordered_targets(config, requested)?;
    let targets = filter_capabilities(ordered, required);
    if targets.is_empty() {
        return Err(GatewayError::InvalidRequest {
            message: format!("aucune cible ne supporte les capacités demandées pour `{requested}`"),
        });
    }
    Ok(targets)
}

fn ordered_targets(config: &CompiledConfig, requested: &str) -> Result<Vec<Target>, GatewayError> {
    if let Some(route) = config.routes.get(requested) {
        return Ok(route
            .fallbacks
            .iter()
            .map(|target| Target::new(target.provider.clone(), target.model.clone()))
            .collect());
    }

    if let Some((provider, model)) = requested.split_once('/') {
        if provider.is_empty() || model.is_empty() {
            return Err(GatewayError::InvalidRequest {
                message: format!("cible invalide : `{requested}`"),
            });
        }
        if !config.providers.contains_key(provider) {
            return Err(GatewayError::InvalidRequest {
                message: format!("fournisseur inconnu : `{provider}`"),
            });
        }
        return Ok(vec![Target::new(provider, model)]);
    }

    Err(GatewayError::InvalidRequest {
        message: format!("alias ou cible inconnue : `{requested}`"),
    })
}

/// Élimine les cibles dont les capacités connues ne couvrent pas `required`.
pub fn filter_capabilities(targets: Vec<Target>, required: Capabilities) -> Vec<Target> {
    targets
        .into_iter()
        .filter(|target| match target.capabilities {
            Some(capabilities) => capabilities.supports(required),
            None => true,
        })
        .collect()
}
