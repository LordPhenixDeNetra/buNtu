//! Capacités d'un modèle, partagées par le routage et les fournisseurs.

/// Capacités qu'un modèle peut supporter.
///
/// Tant que le catalogue de modèles (T2.6) n'existe pas, un fournisseur peut renvoyer
/// `Capabilities::NONE` ou la valeur `Unknown` de son propre catalogue ; le filtre de routage
/// ne supprime que les cibles dont les capacités sont explicitement connues comme insuffisantes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Capabilities {
    pub tools: bool,
    pub vision: bool,
}

impl Capabilities {
    pub const NONE: Self = Self {
        tools: false,
        vision: false,
    };

    /// Vrai si `self` couvre toutes les capacités exigées.
    pub fn supports(self, required: Self) -> bool {
        (!required.tools || self.tools) && (!required.vision || self.vision)
    }
}
