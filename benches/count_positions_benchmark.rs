//! Enhanced benchmarks for position counting with detailed breakdowns.
//!
//! This benchmark tracks position counting performance across
//! different depths and starting positions.

use chess::board::color::Color;
use chess::chess_position;
use chess::prelude::Piece;
use chess::{board::Board, move_generator::MoveGenerator};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

/// Test positions for position counting benchmarks.
fn test_positions() -> Vec<(String, Board)> {
    vec![
        ("starting".to_string(), Board::default()),
        (
            "tactical".to_string(),
            chess_position! {
                ....r..k
                ....q...
                ........
                ........
                ........
                ........
                .....PPP
                R.....K.
            },
        ),
        (
            "endgame".to_string(),
            chess_position! {
                ........
                ........
                ........
                ........
                ........
                ........
                K.......
                .......k
            },
        ),
    ]
}

/// Benchmarks position counting at different depths.
fn benchmark_position_counting(c: &mut Criterion) {
    let mut group = c.benchmark_group("Position Counting");

    for depth in [2, 3, 4, 5] {
        for (name, board) in test_positions() {
            group.bench_with_input(
                BenchmarkId::new(format!("depth_{}_{}", depth, name), depth),
                &depth,
                |b, &depth| {
                    b.iter(|| {
                        let move_generator = MoveGenerator::default();
                        let mut board = board.clone();
                        let count = move_generator.count_positions(
                            depth,
                            black_box(&mut board),
                            Color::White,
                        );
                        black_box(count)
                    })
                },
            );
        }
    }

    group.finish();
}

/// Benchmarks position counting throughput (positions per second).
fn benchmark_position_counting_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("Position Counting Throughput");

    let move_generator = MoveGenerator::default();

    // Measure how many positions can be counted per second at different depths
    for depth in [2, 3, 4] {
        group.bench_function(format!("depth_{}_throughput", depth), |b| {
            b.iter(|| {
                let mut board = Board::default();
                let count =
                    move_generator.count_positions(depth, black_box(&mut board), Color::White);
                black_box(count)
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_position_counting,
    benchmark_position_counting_throughput
);
criterion_main!(benches);
