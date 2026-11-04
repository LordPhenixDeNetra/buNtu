//! Traduction des embeddings OpenAI (`POST /v1/embeddings`).
//!
//! Référence : <https://platform.openai.com/docs/api-reference/embeddings> (consultée le
//! 2026-11-04).

use buntu_schema::{
    Embedding, EmbeddingUsage, EmbeddingsRequest, EmbeddingsResponse, GatewayError,
};
use bytes::Bytes;
use serde_json::{Value, json};

pub const DEFAULT_PATH: &str = "/v1/embeddings";

pub fn build(
    path: &str,
    request: &EmbeddingsRequest,
) -> Result<hyper::Request<Bytes>, GatewayError> {
    let mut body = serde_json::Map::new();
    body.insert("model".to_owned(), Value::String(request.model.clone()));
    let input = if request.input.len() == 1 {
        Value::String(request.input[0].clone())
    } else {
        Value::Array(
            request
                .input
                .iter()
                .map(|text| Value::String(text.clone()))
                .collect(),
        )
    };
    body.insert("input".to_owned(), input);
    if let Some(dimensions) = request.dimensions {
        body.insert("dimensions".to_owned(), json!(dimensions));
    }
    if let Some(format) = &request.encoding_format {
        body.insert("encoding_format".to_owned(), Value::String(format.clone()));
    }
    for (key, value) in &request.extra {
        body.entry(key.clone()).or_insert_with(|| value.clone());
    }
    let body =
        serde_json::to_vec(&Value::Object(body)).map_err(|error| GatewayError::Internal {
            message: format!("requête d'embeddings OpenAI : {error}"),
        })?;
    hyper::Request::builder()
        .method(hyper::Method::POST)
        .uri(path)
        .header("content-type", "application/json")
        .body(Bytes::from(body))
        .map_err(|error| GatewayError::Internal {
            message: format!("requête HTTP d'embeddings : {error}"),
        })
}

pub fn parse(body: &[u8]) -> Result<EmbeddingsResponse, GatewayError> {
    let value: Value = serde_json::from_slice(body).map_err(|error| GatewayError::Internal {
        message: format!("réponse d'embeddings OpenAI illisible : {error}"),
    })?;
    let data =
        value
            .get("data")
            .and_then(Value::as_array)
            .ok_or_else(|| GatewayError::Internal {
                message: "réponse d'embeddings sans `data`".to_owned(),
            })?;
    let mut embeddings = Vec::with_capacity(data.len());
    for (position, item) in data.iter().enumerate() {
        let embedding = item
            .get("embedding")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_f64)
                    .map(|value| value as f32)
                    .collect::<Vec<f32>>()
            })
            .unwrap_or_default();
        embeddings.push(Embedding {
            index: item
                .get("index")
                .and_then(Value::as_u64)
                .unwrap_or(position as u64) as u32,
            embedding,
            object: item
                .get("object")
                .and_then(Value::as_str)
                .unwrap_or("embedding")
                .to_owned(),
        });
    }
    Ok(EmbeddingsResponse {
        model: value
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        data: embeddings,
        usage: EmbeddingUsage {
            prompt_tokens: value
                .get("usage")
                .and_then(|usage| usage.get("prompt_tokens"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            total_tokens: value
                .get("usage")
                .and_then(|usage| usage.get("total_tokens"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
        },
    })
}
