// benches/optimization_benchmark.rs
//
// Benchmark suite specifically for measuring alpha-beta search optimizations.
// Uses depth 3 for fast iteration during development while still providing
// meaningful performance comparisons.

use chess::{
    alpha_beta_searcher::SearchContext,
    board::{castle_rights::CastleRights, color::Color, piece::Piece, Board},
    chess_position,
    chess_search::search_best_move,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn benchmark_positions() -> Vec<(String, Board)> {
    vec![
        // Starting position - tests opening search with all pieces
        ("starting".to_string(), {
            let mut board = Board::default();
            board.lose_castle_rights(CastleRights::all());
            board
        }),
        // Tactical position - mate in 2 for Black, tests tactical acuity
        ("tactical_mate_in_2".to_string(), {
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
            board.set_turn(Color::Black);
            board.lose_castle_rights(CastleRights::all());
            board
        }),
        // Quiet middlegame - no immediate tactics, tests positional search
        ("quiet_middlegame".to_string(), {
            let mut board = chess_position! {
                r...k..r
                ppp..ppp
                ..n.bn..
                ...pp...
                ...PP...
                ..N.BN..
                PPP..PPP
                R...K..R
            };
            board.lose_castle_rights(CastleRights::all());
            board
        }),
        // Complex middlegame with queens - tests branching factor handling
        ("complex_middlegame".to_string(), {
            let mut board = chess_position! {
                r..q.rk.
                ppp.bppp
                ..n.pn..
                ........
                ........
                ..N.PN..
                PPP.BPPP
                R..Q.RK.
            };
            board.lose_castle_rights(CastleRights::all());
            board
        }),
        // Endgame - fewer pieces, deeper calculation possible
        ("endgame".to_string(), {
            let mut board = chess_position! {
                ........
                ........
                ...k....
                ........
                ...K....
                ........
                .R......
                ........
            };
            board.set_turn(Color::White);
            board.lose_castle_rights(CastleRights::all());
            board
        }),
    ]
}

fn optimization_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Search Optimizations");

    // Test both parallel and sequential modes
    let parallel_modes = [("seq", false), ("par", true)];

    // Use depth 4 and 5 to see optimization benefits
    for depth in [4, 5] {
        for (name, initial_board) in benchmark_positions() {
            for (mode_suffix, parallel) in parallel_modes {
                let bench_name = format!("{}_{}_depth_{}", name, mode_suffix, depth);

                group.bench_with_input(
                    BenchmarkId::new(&bench_name, depth),
                    &depth,
                    |b, &depth| {
                        b.iter_batched(
                            || {
                                // Setup for each iteration
                                let board = initial_board.clone();
                                let context = SearchContext::with_parallel(depth, parallel);
                                (board, context)
                            },
                            |(mut board, mut context)| {
                                // The actual search being measured
                                let result = search_best_move(&mut context, &mut board);

                                // Print metrics after search completes
                                if let Ok(best_move) = &result {
                                    eprintln!(
                                "[{} {}] depth={} move={:?} nodes={} score={:?} duration={:?}",
                                name,
                                mode_suffix,
                                depth,
                                best_move,
                                context.searched_position_count(),
                                context.last_score(),
                                context.last_search_duration()
                            );
                                }

                                black_box(result.unwrap())
                            },
                            criterion::BatchSize::LargeInput,
                        )
                    },
                );
            }
        }
    }

    group.finish();
}

criterion_group!(benches, optimization_benchmark);
criterion_main!(benches);
