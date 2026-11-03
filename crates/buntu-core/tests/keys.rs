//! Tests de la sélection pondérée des clés.

use std::error::Error;
use std::time::{Duration, Instant};

use buntu_core::keys::{KeySelectionError, KeySelector};

fn weights() -> [u32; 4] {
    [1, 2, 3, 4]
}

#[test]
fn statistical_distribution_matches_weights() -> Result<(), Box<dyn Error>> {
    let selector = KeySelector::new(&weights())?;
    let draws = 1_000_000_u32;
    let mut counts = [0_u64; 4];
    for _ in 0..draws {
        counts[selector.select()?] += 1;
    }

    let total = f64::from(draws);
    let expected = [0.1_f64, 0.2, 0.3, 0.4];
    for (index, expected_share) in expected.iter().enumerate() {
        let observed = counts[index] as f64 / total;
        assert!(
            (observed - expected_share).abs() < 0.01,
            "clé {index} : {observed:.4} au lieu de {expected_share:.4}"
        );
    }
    Ok(())
}

#[test]
fn paused_key_is_never_selected() -> Result<(), Box<dyn Error>> {
    let selector = KeySelector::new(&[1, 1])?;
    selector.pause(0, Duration::from_millis(200))?;

    for _ in 0..10_000 {
        assert_eq!(selector.select()?, 1, "une clé en pause a été sélectionnée");
    }

    selector.unpause(0)?;
    let mut seen = false;
    for _ in 0..10_000 {
        if selector.select()? == 0 {
            seen = true;
            break;
        }
    }
    assert!(seen, "la clé n'est jamais revenue après la levée de pause");
    Ok(())
}

#[test]
fn all_paused_returns_a_clear_error() -> Result<(), Box<dyn Error>> {
    let selector = KeySelector::new(&[1, 3])?;
    selector.pause(0, Duration::from_secs(5))?;
    selector.pause(1, Duration::from_secs(10))?;

    match selector.select() {
        Err(KeySelectionError::AllPaused { retry_after }) => {
            assert!(retry_after > Duration::ZERO);
            assert!(retry_after <= Duration::from_secs(5));
        }
        other => panic!("erreur attendue, obtenu : {other:?}"),
    }
    Ok(())
}

#[test]
fn construction_rejects_empty_and_zero_weights() {
    assert!(matches!(
        KeySelector::new(&[]),
        Err(KeySelectionError::Empty)
    ));
    assert!(matches!(
        KeySelector::new(&[1, 0]),
        Err(KeySelectionError::ZeroWeight)
    ));
}

#[test]
fn pause_rejects_an_unknown_index() -> Result<(), Box<dyn Error>> {
    let selector = KeySelector::new(&[1])?;
    assert!(matches!(
        selector.pause(3, Duration::from_secs(1)),
        Err(KeySelectionError::IndexOutOfRange { .. })
    ));
    Ok(())
}

#[test]
fn selection_is_under_fifty_nanoseconds_in_release() -> Result<(), Box<dyn Error>> {
    let selector = KeySelector::new(&weights())?;
    let mut accumulator = 0_usize;
    for _ in 0..10_000 {
        accumulator += selector.select()?;
    }

    let iterations = 1_000_000_u32;
    let started = Instant::now();
    for _ in 0..iterations {
        accumulator += selector.select()?;
    }
    let elapsed = started.elapsed();
    std::hint::black_box(accumulator);

    let per_iteration = elapsed.as_nanos() as f64 / f64::from(iterations);
    println!("sélection : {per_iteration:.1} ns par tirage");
    let limit = if cfg!(debug_assertions) { 500.0 } else { 50.0 };
    assert!(
        per_iteration < limit,
        "{per_iteration:.1} ns par tirage, limite {limit:.0} ns"
    );
    Ok(())
}
