//! Tests de la politique de réessais.

use std::error::Error;
use std::time::Duration;

use buntu_core::retry::{
    Backoff, ErrorClass, RetryBudget, classify, classify_status, is_retryable,
};
use buntu_schema::GatewayError;

#[test]
fn statuses_are_classified_per_the_specification() {
    for status in [429_u16, 500, 502, 503, 504] {
        assert_eq!(
            classify_status(status),
            ErrorClass::Recoverable,
            "statut {status}"
        );
    }
    for status in [400_u16, 401, 403, 404, 422] {
        assert_eq!(
            classify_status(status),
            ErrorClass::NonRecoverable,
            "statut {status}"
        );
    }
    assert_eq!(classify_status(418), ErrorClass::NonRecoverable);
}

#[test]
fn errors_are_classified_by_variant() {
    let recoverable = [
        GatewayError::ProviderRateLimited { retry_after: None },
        GatewayError::ProviderUnavailable {
            message: "503".to_owned(),
        },
        GatewayError::Timeout,
    ];
    for error in &recoverable {
        assert_eq!(classify(error), ErrorClass::Recoverable, "erreur {error}");
    }

    let non_recoverable = [
        GatewayError::InvalidRequest {
            message: "400".to_owned(),
        },
        GatewayError::Authentication,
        GatewayError::BudgetExceeded,
        GatewayError::AllTargetsFailed {
            attempts: vec!["openai/gpt-4o-mini".to_owned()],
            last: Box::new(GatewayError::Timeout),
        },
        GatewayError::Internal {
            message: "bug".to_owned(),
        },
    ];
    for error in &non_recoverable {
        assert_eq!(
            classify(error),
            ErrorClass::NonRecoverable,
            "erreur {error}"
        );
    }
}

#[test]
fn a_started_stream_is_never_retried() {
    let unavailable = GatewayError::ProviderUnavailable {
        message: "503".to_owned(),
    };
    assert!(is_retryable(&unavailable, false));
    assert!(!is_retryable(&unavailable, true));

    let invalid = GatewayError::InvalidRequest {
        message: "400".to_owned(),
    };
    assert!(!is_retryable(&invalid, false));
}

#[test]
fn backoff_grows_then_is_capped_and_fully_jittered() {
    let backoff = Backoff::new(Duration::from_millis(100), Duration::from_secs(8));

    for _ in 0..1_000 {
        assert!(backoff.delay(0) <= Duration::from_millis(100));
    }

    let mut minimum = Duration::MAX;
    let mut maximum = Duration::ZERO;
    for _ in 0..1_000 {
        let delay = backoff.delay(10);
        assert!(
            delay <= Duration::from_secs(8),
            "plafond dépassé : {delay:?}"
        );
        minimum = minimum.min(delay);
        maximum = maximum.max(delay);
    }
    assert!(
        maximum > Duration::from_secs(7),
        "gigue trop faible : max {maximum:?}"
    );
    assert!(
        minimum < Duration::from_millis(800),
        "gigue non complète : min {minimum:?}"
    );
}

#[test]
fn retry_rate_is_capped_at_twenty_percent() -> Result<(), Box<dyn Error>> {
    let budget = RetryBudget::new(20, 20);
    let requests = 20_000_u32;
    let mut retries = 0_u32;

    for _ in 0..requests {
        budget.record_request();
        // Scénario de la fiche : 100 % d'erreurs 503, toutes classées récupérables.
        if classify_status(503) == ErrorClass::Recoverable && budget.try_retry() {
            retries += 1;
        }
    }

    let ratio = f64::from(retries) / f64::from(requests);
    println!(
        "taux de réessai : {:.3} % ({retries} sur {requests})",
        ratio * 100.0
    );
    assert!(ratio <= 0.205, "taux {ratio:.4} supérieur à 20 %");
    assert!(ratio >= 0.18, "taux {ratio:.4} anormalement bas");
    Ok(())
}

#[test]
fn exhausted_budget_refuses_retries() -> Result<(), Box<dyn Error>> {
    let budget = RetryBudget::new(20, 5);
    assert_eq!(budget.available(), 5);

    for _ in 0..5 {
        assert!(budget.try_retry(), "un jeton initial manque");
    }
    assert_eq!(budget.available(), 0);
    for _ in 0..5 {
        assert!(!budget.try_retry(), "le budget devrait être épuisé");
    }

    for _ in 0..5 {
        budget.record_request();
    }
    assert!(
        budget.try_retry(),
        "la requête suivante devrait rendre un jeton"
    );
    assert!(!budget.try_retry());
    Ok(())
}
