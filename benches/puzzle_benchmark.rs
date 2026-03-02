//! Puzzle-solving benchmark measuring both solve speed and accuracy.
//!
//! Three tiers of puzzles test regression (tactical), improvement (strategic),
//! and aspirational (deep positional) capabilities. Solve rates are printed
//! to stderr after each group.

use std::str::FromStr;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use chess::{alpha_beta_searcher::SearchContext, board::Board, chess_search::search_best_move};

/// (FEN, best_move_uci, name, tier)
/// Tier 1 = tactical depth 6, Tier 2 = strategic depth 10, Tier 3 = deep depth 12
struct Puzzle {
    fen: &'static str,
    best_move: &'static str,
    name: &'static str,
    tier: u8,
}

const PUZZLES: &[Puzzle] = &[
    // ==========================================
    // TIER 1: Tactical puzzles (depth 6)
    // ==========================================
    // Mate in 2: Qh7#
    Puzzle {
        fen: "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4",
        best_move: "h5f7",
        name: "scholars_mate_qxf7",
        tier: 1,
    },
    // Back rank mate: Rd1 Re1#
    Puzzle {
        fen: "6k1/5ppp/8/8/8/8/5PPP/3rR1K1 w - - 0 1",
        best_move: "e1d1",
        name: "back_rank_capture",
        tier: 1,
    },
    // Knight fork winning queen: Nc7+
    Puzzle {
        fen: "r1bqk2r/ppppbppp/2n2n2/4N3/4P3/8/PPPP1PPP/RNBQKB1R w KQkq - 0 5",
        best_move: "e5c6",
        name: "knight_fork_royal",
        tier: 1,
    },
    // Pin winning material
    Puzzle {
        fen: "rnbqk2r/ppp2ppp/3p1n2/4p3/1bB1P3/2N2N2/PPPP1PPP/R1BQK2R w KQkq - 0 5",
        best_move: "d2d3",
        name: "defend_against_pin",
        tier: 1,
    },
    // Discovered attack
    Puzzle {
        fen: "r1bqkbnr/pppp1ppp/2n5/4p3/3PP3/5N2/PPP2PPP/RNBQKB1R b KQkq d3 0 3",
        best_move: "e5d4",
        name: "central_pawn_capture",
        tier: 1,
    },
    // WAC.001: Fork winning material
    Puzzle {
        fen: "2rr3k/pp3pp1/1nnrp1p1/3pN3/2pP4/2P3P1/PPB1PP1P/3RR1K1 w - - 0 1",
        best_move: "e5g6",
        name: "WAC_001",
        tier: 1,
    },
    // WAC.003
    Puzzle {
        fen: "5rk1/1ppb3p/p1pb4/6q1/3P1p1r/2P1R2P/PP1BQ1P1/5RK1 w - - 0 1",
        best_move: "e3g3",
        name: "WAC_003",
        tier: 1,
    },
    // WAC.005
    Puzzle {
        fen: "r1b1k2r/ppppnppp/2n2q2/2b5/3NP3/2P1B3/PP3PPP/RN1QKB1R w KQkq - 0 1",
        best_move: "d4f5",
        name: "WAC_005",
        tier: 1,
    },
    // WAC.007: Pawn fork
    Puzzle {
        fen: "3q1rk1/p4pp1/2pb3p/3p4/6Pr/1PNQ4/P1PB1PP1/4RRK1 b - - 0 1",
        best_move: "d6h2",
        name: "WAC_007",
        tier: 1,
    },
    // Simple tactic: queen takes hanging piece
    Puzzle {
        fen: "r1bq1rk1/ppp2ppp/2n1pn2/3p4/1bPP4/2N1PN2/PP3PPP/R1BQKB1R w KQ - 0 6",
        best_move: "c4d5",
        name: "central_exchange",
        tier: 1,
    },
    // Skewer
    Puzzle {
        fen: "8/8/1p1k4/5p2/1P3P2/8/3K1B2/8 w - - 0 1",
        best_move: "f2e3",
        name: "bishop_centralize",
        tier: 1,
    },
    // Mate in 2 with queen
    Puzzle {
        fen: "r4rk1/ppp2ppp/8/8/3q4/3B4/PPPQ1PPP/R4RK1 w - - 0 1",
        best_move: "d3h7",
        name: "bishop_attacks_h7",
        tier: 1,
    },
    // ==========================================
    // TIER 2: Bratko-Kopec strategic (depth 10)
    // ==========================================
    // BK.01 -- d5 pawn break
    Puzzle {
        fen: "1k1r4/pp1b1R2/3q2pp/4p3/2B5/4Q3/PPP2B2/2K5 b - - 0 1",
        best_move: "d6d1",
        name: "BK_01",
        tier: 2,
    },
    // BK.02 -- Piece activity
    Puzzle {
        fen: "3r1k2/4npp1/1ppr3p/p6P/P2PPPP1/1NR5/5K2/2R5 w - - 0 1",
        best_move: "d4d5",
        name: "BK_02",
        tier: 2,
    },
    // BK.03
    Puzzle {
        fen: "2q1rr1k/3bbnnp/p2p1pp1/2pPp3/PpP1P1P1/1P2BNNP/2BQ1PRR/7K b - - 0 1",
        best_move: "f6f5",
        name: "BK_03",
        tier: 2,
    },
    // BK.04
    Puzzle {
        fen: "rnbqkb1r/p3pppp/1p6/2ppP3/3N4/2P5/PPP1BPPP/R1BQK2R w KQkq - 0 1",
        best_move: "e5e6",
        name: "BK_04",
        tier: 2,
    },
    // BK.05
    Puzzle {
        fen: "r1b2rk1/2q1b1pp/p2ppn2/1p6/3QP3/1BN1B3/PPP3PP/R4RK1 w - - 0 1",
        best_move: "c3d5",
        name: "BK_05",
        tier: 2,
    },
    // BK.06
    Puzzle {
        fen: "2r3k1/pppR1pp1/4p1p1/4P3/5P2/1P4P1/P1P3K1/8 w - - 0 1",
        best_move: "d7d4",
        name: "BK_06",
        tier: 2,
    },
    // BK.07
    Puzzle {
        fen: "1nk1r1r1/pp2n1pp/4p3/q2pPp1N/b1pP4/B1P2R1P/2P1BPP1/R2QK3 w Q - 0 1",
        best_move: "h5f4",
        name: "BK_07",
        tier: 2,
    },
    // BK.08
    Puzzle {
        fen: "4b3/p3kp2/6p1/3pP2p/2pP1P2/2P1K1P1/P7/4B3 w - - 0 1",
        best_move: "f4f5",
        name: "BK_08",
        tier: 2,
    },
    // BK.09
    Puzzle {
        fen: "2kr1bnr/pbpq4/2n1pp2/3p3p/3P1P1B/2N2N1Q/PPP3PP/2KR1B1R w - - 0 1",
        best_move: "f4f5",
        name: "BK_09",
        tier: 2,
    },
    // BK.10
    Puzzle {
        fen: "3rr1k1/pp3pp1/1qn2np1/8/3p4/PP1R1P2/2P1NQPP/R1B3K1 b - - 0 1",
        best_move: "c6e5",
        name: "BK_10",
        tier: 2,
    },
    // BK.11
    Puzzle {
        fen: "2r1nrk1/p2q1ppp/bp1p4/n1pPp3/P1P1P3/2PBB1N1/4QPPP/R4RK1 w - - 0 1",
        best_move: "g3f5",
        name: "BK_11",
        tier: 2,
    },
    // BK.12
    Puzzle {
        fen: "r3r1k1/ppqb1ppp/8/4p1NQ/8/2P5/PP3PPP/R3R1K1 b - - 0 1",
        best_move: "d7f5",
        name: "BK_12",
        tier: 2,
    },
    // BK.13
    Puzzle {
        fen: "r2q1rk1/4bppp/p2p4/2pP4/3pP3/3Q4/PP1B1PPP/R3R1K1 w - - 0 1",
        best_move: "b2b4",
        name: "BK_13",
        tier: 2,
    },
    // BK.14
    Puzzle {
        fen: "rnb2r1k/pp2p2p/2pp2p1/q2P1p2/8/1Pb2NP1/PB2PPBP/R2Q1RK1 w - - 0 1",
        best_move: "d1d2",
        name: "BK_14",
        tier: 2,
    },
    // BK.15
    Puzzle {
        fen: "2r4k/2r4p/p7/2b2p1b/4pP2/6P1/P1B1R2P/2B1R1K1 w - - 0 1",
        best_move: "c1e3",
        name: "BK_15",
        tier: 2,
    },
    // BK.16
    Puzzle {
        fen: "r1bqkb1r/4npp1/p1p4p/1p1pP1B1/8/1B6/PPPN1PPP/R2Q1RK1 w kq - 0 1",
        best_move: "d2e4",
        name: "BK_16",
        tier: 2,
    },
    // BK.17
    Puzzle {
        fen: "r2q1rk1/1ppnbppp/p2p1nb1/3Pp3/2P1P1P1/2N2P1P/PPB5/R1BQN1K1 b - - 0 1",
        best_move: "b7b5",
        name: "BK_17",
        tier: 2,
    },
    // BK.18
    Puzzle {
        fen: "r1bq1rk1/pp2ppbp/2np2p1/2n5/P3PP2/N1P2N2/1PB3PP/R1B1QRK1 b - - 0 1",
        best_move: "c5b3",
        name: "BK_18",
        tier: 2,
    },
    // BK.19
    Puzzle {
        fen: "3rr3/2pq2pk/p2p1pnp/8/2QBPP2/1P6/P5PP/4RRK1 b - - 0 1",
        best_move: "e8e4",
        name: "BK_19",
        tier: 2,
    },
    // BK.20
    Puzzle {
        fen: "r4k2/pb2bp1r/1p1qp2p/3pNp2/3P1P2/2N3P1/PPP1Q2P/2KRR3 w - - 0 1",
        best_move: "g3g4",
        name: "BK_20",
        tier: 2,
    },
    // BK.21
    Puzzle {
        fen: "3rn2k/ppb2rpp/2ppqp2/5N2/2P1P3/1P5Q/PB3PPP/3RR1K1 w - - 0 1",
        best_move: "f5h6",
        name: "BK_21",
        tier: 2,
    },
    // BK.22
    Puzzle {
        fen: "2r2rk1/1bqnbpp1/1p1ppn1p/pP6/N1P1P3/P2B1N1P/1B2QPP1/R2R2K1 b - - 0 1",
        best_move: "b7e4",
        name: "BK_22",
        tier: 2,
    },
    // BK.23
    Puzzle {
        fen: "r1bqk2r/pp2bppp/2p5/3pP3/P2Q1P2/2N1B3/1PP3PP/R4RK1 b kq - 0 1",
        best_move: "f7f6",
        name: "BK_23",
        tier: 2,
    },
    // BK.24
    Puzzle {
        fen: "r2qnrnk/p2b2b1/1p1p2pp/2pPpp2/1PP1P3/PRNBB3/3QNPPP/5RK1 w - - 0 1",
        best_move: "f2f4",
        name: "BK_24",
        tier: 2,
    },
    // ==========================================
    // TIER 3: Deep positional (depth 12)
    // ==========================================
    // Rook on 7th rank
    Puzzle {
        fen: "2r2rk1/pp3ppp/8/3pP3/8/1P6/P4PPP/2RR2K1 w - - 0 1",
        best_move: "c1c7",
        name: "rook_to_7th",
        tier: 3,
    },
    // Passed pawn advancement
    Puzzle {
        fen: "8/pp3kpp/2p5/3p4/3P4/2P2P2/PP4PP/6K1 w - - 0 1",
        best_move: "f3f4",
        name: "pawn_advance",
        tier: 3,
    },
    // King activity in endgame
    Puzzle {
        fen: "8/8/1p2k1p1/1P4p1/6P1/4K3/8/8 w - - 0 1",
        best_move: "e3d4",
        name: "king_activity",
        tier: 3,
    },
    // Rook behind passed pawn
    Puzzle {
        fen: "8/5pk1/5Rp1/7p/3r4/6PP/5PK1/8 w - - 0 1",
        best_move: "f6a6",
        name: "rook_activity",
        tier: 3,
    },
    // Bishop pair advantage
    Puzzle {
        fen: "r4rk1/pp2ppbp/3p1np1/q7/3BP3/2N2P2/PPPQ2PP/R4RK1 w - - 0 1",
        best_move: "d4e3",
        name: "bishop_pair_preservation",
        tier: 3,
    },
    // Pawn structure -- create passed pawn
    Puzzle {
        fen: "8/pp1r1pkp/6p1/3Pp3/4P3/6PP/PP3R1K/8 w - - 0 1",
        best_move: "d5d6",
        name: "create_passed_pawn",
        tier: 3,
    },
    // Positional sacrifice
    Puzzle {
        fen: "r2q1rk1/pppbbppp/2n1p3/3pP3/3P4/2N2N2/PPP1BPPP/R1BQ1RK1 w - - 0 9",
        best_move: "f3g5",
        name: "positional_pressure",
        tier: 3,
    },
    // Open file control
    Puzzle {
        fen: "r4rk1/1b2bppp/ppnppn2/q7/2P1P3/P1N1BN2/1PQ2PPP/R4RK1 w - - 0 13",
        best_move: "f1d1",
        name: "open_file_control",
        tier: 3,
    },
];

fn solve_puzzle(puzzle: &Puzzle, depth: u8) -> bool {
    let mut board = Board::from_str(puzzle.fen).expect("Invalid FEN");
    let mut context = SearchContext::new(depth);
    match search_best_move(&mut context, &mut board) {
        Ok(best) => best.to_uci() == puzzle.best_move,
        Err(_) => false,
    }
}

fn run_tier(
    c: &mut Criterion,
    tier: u8,
    depth: u8,
    group_name: &str,
    sample_size: usize,
    measurement_secs: u64,
) {
    let puzzles: Vec<&Puzzle> = PUZZLES.iter().filter(|p| p.tier == tier).collect();

    // Print solve rate once before benchmarking (avoids solving every puzzle twice)
    let mut solved = 0;
    for puzzle in &puzzles {
        if solve_puzzle(puzzle, depth) {
            solved += 1;
        } else {
            eprintln!(
                "  MISS {}: {} (expected {})",
                group_name, puzzle.name, puzzle.best_move
            );
        }
    }
    eprintln!("{}: {}/{} solved", group_name, solved, puzzles.len());

    // Criterion timing benchmark
    let mut group = c.benchmark_group(group_name);
    group.sample_size(sample_size);
    group.measurement_time(std::time::Duration::from_secs(measurement_secs));
    group.bench_function("solve_all", |b| {
        b.iter(|| {
            let mut count = 0u32;
            for puzzle in &puzzles {
                if solve_puzzle(puzzle, depth) {
                    count += 1;
                }
            }
            black_box(count)
        })
    });
    group.finish();
}

fn puzzle_benchmark(c: &mut Criterion) {
    run_tier(c, 1, 6, "puzzles_tactical_d6", 20, 10);
    run_tier(c, 2, 10, "puzzles_strategic_d10", 10, 30);
    run_tier(c, 3, 12, "puzzles_deep_d12", 10, 60);
}

criterion_group!(benches, puzzle_benchmark);
criterion_main!(benches);
