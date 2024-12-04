// benches/alpha_beta_history_benchmark.rs
use chess::{
    alpha_beta_searcher::{alpha_beta_search, SearchContext},
    board::{castle_rights_bitmask::ALL_CASTLE_RIGHTS, color::Color, piece::Piece, Board},
    chess_position,
    move_generator::MoveGenerator,
};
use common::bitboard::bitboard::Bitboard;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn benchmark_positions() -> Vec<(String, Board)> {
    vec![
        // Position that tests tactical play
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
        // Position that tests quiet positional play
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
        // Complex middlegame position
        (
            "middlegame".to_string(),
            chess_position! {
                r..q.rk.
                ppp..ppp
                ..n.....
                ....p...
                ....P...
                ........
                PPP..PPP
                R..Q.RK.
            },
        ),
    ]
}

fn alpha_beta_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Alpha-Beta Search");

    // Test different search depths
    for depth in [4, 5] {
        // Test each position
        for (name, mut initial_board) in benchmark_positions() {
            initial_board.lose_castle_rights(ALL_CASTLE_RIGHTS);
            group.bench_with_input(
                BenchmarkId::new(format!("{}_depth_{}", name, depth), depth),
                &depth,
                |b, &depth| {
                    b.iter_batched(
                        || {
                            // Setup for each iteration
                            let board = initial_board.clone();
                            let move_gen = MoveGenerator::default();
                            let context = SearchContext::new(depth);
                            (board, move_gen, context)
                        },
                        |(mut board, mut move_gen, mut context)| {
                            // The actual search
                            black_box(
                                alpha_beta_search(&mut context, &mut board, &mut move_gen).unwrap(),
                            )
                        },
                        criterion::BatchSize::LargeInput,
                    )
                },
            );
        }
    }

    group.finish();
}

criterion_group!(benches, alpha_beta_benchmark);
criterion_main!(benches);
