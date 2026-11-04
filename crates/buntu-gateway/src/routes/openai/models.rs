//! Endpoint `/v1/models` (T1.13) : alias et cibles configurés.

use std::collections::BTreeSet;

use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};

use crate::state::GatewayState;

pub async fn models(State(state): State<GatewayState>) -> Json<Value> {
    let config = state.config.snapshot();
    let mut ids: BTreeSet<String> = BTreeSet::new();
    for (alias, route) in &config.routes {
        ids.insert(alias.clone());
        for target in &route.fallbacks {
            ids.insert(format!("{}/{}", target.provider, target.model));
        }
    }
    let data: Vec<Value> = ids
        .into_iter()
        .map(|id| {
            json!({
                "id": id,
                "object": "model",
                "created": 0,
                "owned_by": "buntu",
            })
        })
        .collect();
    Json(json!({ "object": "list", "data": data }))
}
