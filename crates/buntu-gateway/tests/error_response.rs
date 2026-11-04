//! Tests instantanés (insta) des erreurs normalisées T1.15.

use std::time::Duration;

use axum::body::to_bytes;
use buntu_gateway::error_response;
use buntu_schema::GatewayError;
use serde_json::Value;

async fn rendered(error: GatewayError) -> (u16, Option<String>, Option<String>, Value) {
    let response = error_response(&error);
    let status = response.status().as_u16();
    let source = response
        .headers()
        .get("x-buntu-error-source")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let retry_after = response
        .headers()
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap_or_default();
    let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    (status, source, retry_after, json)
}

#[tokio::test]
async fn provider_rate_limited() {
    let (status, source, retry_after, body) = rendered(GatewayError::ProviderRateLimited {
        retry_after: Some(Duration::from_secs(2)),
    })
    .await;
    assert_eq!(status, 429);
    assert_eq!(source.as_deref(), Some("provider"));
    assert_eq!(retry_after.as_deref(), Some("2"));
    insta::assert_json_snapshot!(body);
}

#[tokio::test]
async fn provider_unavailable() {
    let (status, source, _, body) = rendered(GatewayError::ProviderUnavailable {
        message: "statut 503".to_owned(),
    })
    .await;
    assert_eq!(status, 502);
    assert_eq!(source.as_deref(), Some("provider"));
    insta::assert_json_snapshot!(body);
}

#[tokio::test]
async fn timeout() {
    let (status, source, _, body) = rendered(GatewayError::Timeout).await;
    assert_eq!(status, 504);
    assert_eq!(source.as_deref(), Some("provider"));
    insta::assert_json_snapshot!(body);
}

#[tokio::test]
async fn invalid_request() {
    let (status, source, _, body) = rendered(GatewayError::InvalidRequest {
        message: "cible inconnue".to_owned(),
    })
    .await;
    assert_eq!(status, 400);
    assert_eq!(source.as_deref(), Some("client"));
    insta::assert_json_snapshot!(body);
}

#[tokio::test]
async fn authentication() {
    let (status, source, _, body) = rendered(GatewayError::Authentication).await;
    assert_eq!(status, 401);
    assert_eq!(source.as_deref(), Some("client"));
    insta::assert_json_snapshot!(body);
}

#[tokio::test]
async fn budget_exceeded() {
    let (status, source, _, body) = rendered(GatewayError::BudgetExceeded).await;
    assert_eq!(status, 402);
    assert_eq!(source.as_deref(), Some("budget"));
    insta::assert_json_snapshot!(body);
}

#[tokio::test]
async fn all_targets_failed() {
    let (status, source, _, body) = rendered(GatewayError::AllTargetsFailed {
        attempts: vec!["openai/gpt-4o-mini (2 tentative(s))".to_owned()],
        last: Box::new(GatewayError::ProviderUnavailable {
            message: "statut 503".to_owned(),
        }),
    })
    .await;
    assert_eq!(status, 502);
    assert_eq!(source.as_deref(), Some("provider"));
    insta::assert_json_snapshot!(body);
}

#[tokio::test]
async fn internal_error() {
    let (status, source, _, body) = rendered(GatewayError::Internal {
        message: "bug".to_owned(),
    })
    .await;
    assert_eq!(status, 500);
    assert_eq!(source.as_deref(), Some("gateway"));
    insta::assert_json_snapshot!(body);
}

#[tokio::test]
async fn messages_never_contain_secrets_or_request_bodies() {
    let errors = [
        GatewayError::InvalidRequest {
            message: "paramètre invalide".to_owned(),
        },
        GatewayError::ProviderUnavailable {
            message: "statut 503".to_owned(),
        },
        GatewayError::AllTargetsFailed {
            attempts: vec!["openai/gpt-4o-mini (1 tentative(s))".to_owned()],
            last: Box::new(GatewayError::Timeout),
        },
    ];
    for error in &errors {
        let (_, _, _, body) = rendered(match error {
            GatewayError::InvalidRequest { message } => GatewayError::InvalidRequest {
                message: message.clone(),
            },
            GatewayError::ProviderUnavailable { message } => GatewayError::ProviderUnavailable {
                message: message.clone(),
            },
            other => GatewayError::Internal {
                message: other.to_string(),
            },
        })
        .await;
        let text = body.to_string();
        assert!(!text.contains("sk-"), "message : {text}");
        assert!(!text.contains("Authorization"), "message : {text}");
        assert!(!text.contains("Bearer"), "message : {text}");
    }
}
