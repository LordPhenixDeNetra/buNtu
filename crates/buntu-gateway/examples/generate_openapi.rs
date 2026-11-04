//! Génère `openapi.json` à la racine du dépôt.
//!
//! Usage : `cargo run -p buntu-gateway --example generate_openapi > openapi.json`
//! Le test `tests/openapi.rs` échoue si le fichier commité n'est plus à jour.

use utoipa::OpenApi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let specification = buntu_gateway::admin::AdminApi::openapi();
    println!("{}", serde_json::to_string_pretty(&specification)?);
    Ok(())
}
