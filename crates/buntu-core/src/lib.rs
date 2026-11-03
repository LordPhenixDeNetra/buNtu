//! Cœur de buNtu : configuration compilée, contexte de requête, hooks, pipeline, sélection
//! pondérée des clés, politique de réessais et chaîne de repli.

pub mod config;
pub mod context;
pub mod fallback;
pub mod hooks;
pub mod keys;
pub mod pipeline;
pub mod retry;
pub mod routing;
