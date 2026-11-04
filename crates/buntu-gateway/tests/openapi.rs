//! Vérifie que `openapi.json` (commité) correspond à la spécification générée (T1.16).

use utoipa::OpenApi;

fn normalize(text: &str) -> String {
    text.lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn openapi_json_is_up_to_date() {
    let generated = serde_json::to_string_pretty(&buntu_gateway::admin::AdminApi::openapi())
        .unwrap_or_default();
    let committed =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../openapi.json"));
    let committed = match committed {
        Ok(text) => text,
        Err(error) => panic!("openapi.json illisible : {error}"),
    };
    assert_eq!(
        normalize(&committed),
        normalize(&generated),
        "openapi.json est obsolète : relancez `cargo run -p buntu-gateway --example generate_openapi > openapi.json`"
    );
}
