use chess::board::color::Color;
use chess::{board::Board, move_generator::MoveGenerator};

use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("count all possible positions to depth 4", |b| {
        b.iter(|| {
            let mut move_generator = MoveGenerator::new();
            move_generator.count_positions(4, &mut Board::default(), Color::White)
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
