//! Kit de conformité des fournisseurs (T1.8).
//!
//! Une batterie commune de cas (texte, système, flux, outils, image, JSON structuré, arrêts,
//! erreurs HTTP) rejoue des fixtures enregistrées dans `tests/fixtures/<fournisseur>/` via
//! `wiremock`, puis vérifie la traduction canonique produite par le fournisseur.
//!
//! La macro [`conformance_suite!`] génère un test par cas :
//!
//! ```ignore
//! conformance_suite!(MonFournisseur::default(), "mon-fournisseur", text_simple, tool_call);
//! ```
//!
//! Le mode `--live` (variable `BUNTU_LIVE=1`) est prévu pour T1.9 : la même batterie pourra être
//! jouée contre la vraie API, la nuit, sans changer les attentes.

pub mod cases;
pub mod fake;
pub mod fixtures;
pub mod runner;

pub use cases::{Case, CaseKind, cases, find};
pub use fake::{BrokenProvider, FakeProvider};
pub use runner::{block_on_case, live_enabled, run_case};

/// Génère un test par cas de conformité.
#[macro_export]
macro_rules! conformance_suite {
    ($provider:expr, $provider_dir:literal, $($case:ident),+ $(,)?) => {
        $(
            #[test]
            fn $case() -> Result<(), String> {
                $crate::runner::block_on_case(&$provider, $provider_dir, stringify!($case))
            }
        )+
    };
}
