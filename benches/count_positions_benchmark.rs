use chess::board::Board;
use chess::board::color::Color;
use chess::moves::ray_table::RayTable;
use chess::moves::count_positions;

use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    let mut board = Board::starting_position();
    let mut ray_table = RayTable::new();
    let starting_color = Color::White;
    let depth = 4;

    ray_table.populate();

    c.bench_function("count all possible positions to depth 4", |b| {
        b.iter(|| count_positions(depth, &mut board, &ray_table, starting_color))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
