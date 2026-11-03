//! Schéma canonique de buNtu (T0.5).
//!
//! Format interne unique : toute requête est traduite une fois vers ces types, puis vers le
//! fournisseur visé. Les champs qu'un fournisseur ajoute et que nous ne modélisons pas encore
//! sont conservés dans le champ `extra` (passthrough), jamais perdus.
//!
//! Aucun code fournisseur ne vit ici : seulement les types partagés. Les tableaux de
//! correspondance et les sources officielles sont dans `docs/adr/0002-schema.md`.

pub mod capabilities;
pub mod content;
pub mod error;
pub mod message;
pub mod request;
pub mod response;
pub mod stop;
pub mod stream;
pub mod tool;
pub mod usage;

pub use capabilities::Capabilities;
pub use content::{ContentPart, ImageDetail, ImageSource};
pub use error::GatewayError;
pub use message::{Message, Role};
pub use request::{Request, ResponseFormat, ToolChoice};
pub use response::{Choice, Response};
pub use stop::StopReason;
pub use stream::StreamEvent;
pub use tool::{Tool, ToolCall, ToolResult};
pub use usage::Usage;
