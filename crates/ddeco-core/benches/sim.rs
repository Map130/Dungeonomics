//! Simulation benches (criterion). In R0 a stub over mathf so that `cargo build
//! --benches` compiles; replaced with step_day / example-world warm-up in R6.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ddeco_core::mathf;

fn bench_mathf(c: &mut Criterion) {
    c.bench_function("powf", |b| {
        b.iter(|| mathf::powf(black_box(0.8), black_box(-0.5)))
    });
}

criterion_group!(benches, bench_mathf);
criterion_main!(benches);
