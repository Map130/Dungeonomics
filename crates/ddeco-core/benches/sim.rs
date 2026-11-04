//! Бенчи симуляции (criterion). В R0 — заглушка над mathf, чтобы `cargo build
//! --benches` компилировался; в R6 заменяется на step_day/прогрев примера мира.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ddeco_core::mathf;

fn bench_mathf(c: &mut Criterion) {
    c.bench_function("powf", |b| {
        b.iter(|| mathf::powf(black_box(0.8), black_box(-0.5)))
    });
}

criterion_group!(benches, bench_mathf);
criterion_main!(benches);
