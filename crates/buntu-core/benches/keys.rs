//! Benchmark de la sélection pondérée des clés.

use std::hint::black_box;

use buntu_core::keys::KeySelector;
use criterion::{Criterion, criterion_group, criterion_main};

fn selector() -> KeySelector {
    match KeySelector::new(&[1, 2, 3, 4]) {
        Ok(selector) => selector,
        Err(error) => panic!("poids de test invalides : {error}"),
    }
}

fn selection(criterion: &mut Criterion) {
    let selector = selector();
    criterion.bench_function("key_selection_walker", |bench| {
        bench.iter(|| black_box(selector.select()));
    });
}

criterion_group!(benches, selection);
criterion_main!(benches);
