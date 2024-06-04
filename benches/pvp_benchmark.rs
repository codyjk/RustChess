use chess::game::modes;

use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("computer vs computer (depth 3)", |b| {
        b.iter(|| modes::computer_vs_computer(25, 0, 3))
    });

    c.bench_function("computer vs computer (depth 4)", |b| {
        b.iter(|| modes::computer_vs_computer(10, 0, 4))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
