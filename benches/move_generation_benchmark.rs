//! Comprehensive benchmarks for move generation performance.
//!
//! This benchmark suite profiles the critical move generation codepath,
//! including full move generation and individual components where possible.
//! Results are used to track before/after performance improvements.

use chess::board::{castle_rights::CastleRights, color::Color, Board};
use chess::chess_position;
use chess::move_generator::{
    targets::{generate_pawn_attack_targets, generate_pawn_move_targets},
    MoveGenerator, PieceTargetList, Targets,
};
use chess::prelude::Piece;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

/// Test positions representing different game phases and move complexity.
fn benchmark_positions() -> Vec<(String, Board)> {
    let mut positions = vec![
        // Starting position - typical opening position
        ("starting".to_string(), Board::default()),
        // Tactical position - many captures and checks
        ("tactical".to_string(), {
            let mut board = chess_position! {
                ....r..k
                ....q...
                ........
                ........
                ........
                ........
                .....PPP
                R.....K.
            };
            // Disable castling to avoid invalid states
            board.lose_castle_rights(CastleRights::all());
            board
        }),
        // Positional/quiet position - few captures
        (
            "positional".to_string(),
            chess_position! {
                ........
                pp...ppp
                ....p...
                ...p....
                ...P....
                ........
                PPP..PPP
                ........
            },
        ),
        // Complex middlegame - many pieces, complex interactions
        ("middlegame".to_string(), {
            let mut board = chess_position! {
                r..q.rk.
                ppp..ppp
                ..n.....
                ....p...
                ....P...
                ........
                PPP..PPP
                R..Q.RK.
            };
            // Disable castling to avoid invalid states
            board.lose_castle_rights(CastleRights::all());
            board
        }),
        // Endgame position - few pieces
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
        // Position with many pawn moves
        (
            "pawn_heavy".to_string(),
            chess_position! {
                ........
                pppppppp
                ........
                ........
                ........
                ........
                PPPPPPPP
                ........
            },
        ),
    ];

    // Disable castling for all non-starting positions to avoid invalid states
    for (name, board) in &mut positions {
        if name != "starting" {
            board.lose_castle_rights(CastleRights::all());
        }
    }

    positions
}

/// Benchmarks full move generation for different positions.
fn benchmark_full_move_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Full Move Generation");
    // Use fewer samples for faster benchmarking during development
    group.sample_size(10);
    group.warm_up_time(std::time::Duration::from_millis(100));
    group.measurement_time(std::time::Duration::from_secs(1));

    for (name, mut board) in benchmark_positions() {
        for color in [Color::White, Color::Black] {
            board.set_turn(color);
            let move_generator = MoveGenerator::default();

            group.bench_with_input(
                BenchmarkId::new(format!("{}_{:?}", name, color), &name),
                &name,
                |b, _| {
                    b.iter(|| {
                        let moves = move_generator.generate_moves(black_box(&mut board), color);
                        black_box(moves)
                    })
                },
            );
        }
    }

    group.finish();
}

/// Benchmarks move generation with effect calculation (check/checkmate detection).
fn benchmark_move_generation_with_effects(c: &mut Criterion) {
    let mut group = c.benchmark_group("Move Generation with Effects");

    for (name, mut board) in benchmark_positions() {
        for color in [Color::White, Color::Black] {
            board.set_turn(color);
            let move_generator = MoveGenerator::default();

            group.bench_with_input(
                BenchmarkId::new(format!("{}_{:?}", name, color), &name),
                &name,
                |b, _| {
                    b.iter(|| {
                        let moves = move_generator
                            .generate_moves_and_lazily_update_chess_move_effects(
                                black_box(&mut board),
                                color,
                            );
                        black_box(moves)
                    })
                },
            );
        }
    }

    group.finish();
}

/// Benchmarks attack target generation (used for castling, validation, etc.).
fn benchmark_attack_targets(c: &mut Criterion) {
    let mut group = c.benchmark_group("Attack Target Generation");

    let targets = Targets::default();

    for (name, board) in benchmark_positions() {
        for color in [Color::White, Color::Black] {
            group.bench_with_input(
                BenchmarkId::new(format!("{}_{:?}", name, color), &name),
                &name,
                |b, _| {
                    b.iter(|| {
                        let attack_targets =
                            targets.generate_attack_targets(black_box(&board), color);
                        black_box(attack_targets)
                    })
                },
            );
        }
    }

    group.finish();
}

/// Benchmarks sliding piece target generation (rooks, bishops, queens).
fn benchmark_sliding_targets(c: &mut Criterion) {
    let mut group = c.benchmark_group("Sliding Target Generation");

    let targets = Targets::default();

    for (name, board) in benchmark_positions() {
        for color in [Color::White, Color::Black] {
            group.bench_with_input(
                BenchmarkId::new(format!("{}_{:?}", name, color), &name),
                &name,
                |b, _| {
                    use smallvec::smallvec;
                    b.iter(|| {
                        let mut piece_targets: PieceTargetList = smallvec![];
                        targets.generate_sliding_targets(
                            &mut piece_targets,
                            black_box(&board),
                            color,
                        );
                        black_box(piece_targets)
                    })
                },
            );
        }
    }

    group.finish();
}

/// Benchmarks knight and king target generation from precomputed tables.
fn benchmark_precomputed_targets(c: &mut Criterion) {
    let mut group = c.benchmark_group("Precomputed Target Generation");

    let targets = Targets::default();

    for (name, board) in benchmark_positions() {
        for color in [Color::White, Color::Black] {
            for piece_type in ["knight", "king"] {
                use chess::board::piece::Piece;
                let piece = match piece_type {
                    "knight" => Piece::Knight,
                    "king" => Piece::King,
                    _ => unreachable!(),
                };

                group.bench_with_input(
                    BenchmarkId::new(format!("{}_{}_{:?}", name, piece_type, color), &name),
                    &name,
                    |b, _| {
                        use smallvec::smallvec;
                        b.iter(|| {
                            let mut piece_targets: PieceTargetList = smallvec![];
                            targets.generate_targets_from_precomputed_tables(
                                &mut piece_targets,
                                black_box(&board),
                                color,
                                piece,
                            );
                            black_box(piece_targets)
                        })
                    },
                );
            }
        }
    }

    group.finish();
}

/// Benchmarks pawn move target generation.
fn benchmark_pawn_targets(c: &mut Criterion) {
    let mut group = c.benchmark_group("Pawn Target Generation");

    for (name, board) in benchmark_positions() {
        for color in [Color::White, Color::Black] {
            group.bench_with_input(
                BenchmarkId::new(format!("{}_{:?}", name, color), &name),
                &name,
                |b, _| {
                    use smallvec::smallvec;
                    b.iter(|| {
                        let move_targets = generate_pawn_move_targets(black_box(&board), color);
                        let mut attack_targets: PieceTargetList = smallvec![];
                        generate_pawn_attack_targets(&mut attack_targets, black_box(&board), color);
                        black_box((move_targets, attack_targets))
                    })
                },
            );
        }
    }

    group.finish();
}

/// Benchmarks move generation throughput (moves per second).
/// This is a critical metric for overall engine performance.
fn benchmark_move_generation_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("Move Generation Throughput");

    let move_generator = MoveGenerator::default();
    let mut board = Board::default();

    // Measure how many moves can be generated per second
    group.bench_function("starting_position_throughput", |b| {
        b.iter(|| {
            let mut total_moves = 0;
            for _ in 0..1000 {
                let moves = move_generator.generate_moves(black_box(&mut board), Color::White);
                total_moves += moves.len();
                board.set_turn(Color::Black);
                let moves = move_generator.generate_moves(black_box(&mut board), Color::Black);
                total_moves += moves.len();
                board.set_turn(Color::White);
            }
            black_box(total_moves)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_full_move_generation,
    benchmark_move_generation_with_effects,
    benchmark_attack_targets,
    benchmark_sliding_targets,
    benchmark_precomputed_targets,
    benchmark_pawn_targets,
    benchmark_move_generation_throughput
);
criterion_main!(benches);
