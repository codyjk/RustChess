//! Evaluation speed benchmark measuring raw board_material_score() throughput
//! across a diverse set of positions covering all game phases.

use std::str::FromStr;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use chess::board::Board;
use chess::evaluate::board_material_score;

/// ~70 FEN positions covering openings, middlegame, endgame, pawn structure
/// extremes, and asymmetric material. The eval function is called at every
/// leaf node during search, so this benchmark must represent the full spectrum.
const BENCHMARK_FENS: &[&str] = &[
    // === Openings (10) ===
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    // Sicilian Defense
    "rnbqkbnr/pp1ppppp/8/2p5/4P3/8/PPPP1PPP/RNBQKBNR w KQkq c6 0 2",
    // French Defense
    "rnbqkbnr/pppp1ppp/4p3/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2",
    // Caro-Kann
    "rnbqkbnr/pp1ppppp/2p5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2",
    // Queen's Gambit
    "rnbqkbnr/ppp1pppp/8/3p4/2PP4/8/PP2PPPP/RNBQKBNR b KQkq c3 0 2",
    // King's Indian
    "rnbqkb1r/pppppp1p/5np1/8/2PP4/8/PP2PPPP/RNBQKBNR w KQkq - 0 3",
    // Ruy Lopez
    "r1bqkbnr/pppp1ppp/2n5/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3",
    // Italian Game
    "r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3",
    // English Opening
    "rnbqkbnr/pppppppp/8/8/2P5/8/PP1PPPPP/RNBQKBNR b KQkq c3 0 1",
    // Dutch Defense
    "rnbqkbnr/ppppp1pp/8/5p2/3P4/8/PPP1PPPP/RNBQKBNR w KQkq f6 0 2",
    // === Early middlegame (10) -- moves 10-15, all pieces, castled kings ===
    // Sicilian Najdorf typical
    "r1b1kb1r/1pqn1ppp/p2ppn2/8/3NP3/2N1B3/PPP1BPPP/R2QK2R w KQkq - 0 9",
    // QGD typical
    "r1bq1rk1/pppn1ppp/4pn2/3p4/2PP4/2NQPN2/PP3PPP/R1B2RK1 b - - 5 9",
    // Ruy Lopez middlegame
    "r1bq1rk1/2ppbppp/p1n2n2/1p2p3/4P3/1B3N2/PPPP1PPP/RNBQ1RK1 w - - 0 9",
    // King's Indian middlegame
    "r1bq1rk1/pppn1pbp/3p1np1/4p3/2PPP3/2N2N2/PP2BPPP/R1BQ1RK1 w - - 0 9",
    // Caro-Kann middlegame
    "r1bqkb1r/pp1n1ppp/2p1pn2/3p4/2PP4/2N2N2/PP2PPPP/R1BQKB1R w KQkq - 0 6",
    // Sicilian Dragon
    "r1bq1rk1/pp2ppbp/2np1np1/8/3NP3/2N1BP2/PPPQ2PP/R3KB1R w KQ - 0 10",
    // QGA middlegame
    "r1bq1rk1/p2nbppp/1pp1pn2/3p4/2PP4/1PN1PN2/PB3PPP/R2QKB1R w KQ - 0 9",
    // Pirc Defense middlegame
    "r1bq1rk1/ppp1ppbp/2np1np1/8/3PP3/2N2N2/PPP1BPPP/R1BQ1RK1 w - - 0 8",
    // Symmetrical English
    "r1bqk2r/pppp1ppp/2n2n2/2b1p3/2B1P3/2N2N2/PPPP1PPP/R1BQK2R w KQkq - 4 4",
    // Scotch Game middlegame
    "r1bqk2r/pppp1ppp/2n2n2/2b5/3NP3/8/PPP2PPP/RNBQKB1R w KQkq - 1 5",
    // === Complex middlegame (10) -- imbalanced, pawn islands, open/closed ===
    // Opposite-side castling, tension
    "r2qk2r/pp1bbppp/2nppn2/8/2BPP3/2N2N2/PPP2PPP/R1BQ1RK1 b kq - 0 8",
    // Unbalanced pawns, open center
    "r1bq1rk1/pp3ppp/2n1pn2/2pp4/3P4/P1PBP3/1P3PPP/RNBQ1RK1 b - - 0 9",
    // Exchange sacrifice position
    "r4rk1/pp1qppbp/2np1np1/8/2BPP3/2N2N2/PPP2PPP/R2Q1RK1 w - - 0 11",
    // Closed center, maneuvering
    "r1bq1rk1/pp2bppp/2n1pn2/2ppP3/3P4/2N2N2/PPP1BPPP/R1BQ1RK1 w - - 0 9",
    // Many pawn islands
    "r3kb1r/p3pppp/1qnp1n2/1pp5/3PP3/2N2N2/PPP1BPPP/R1BQK2R w KQkq - 0 8",
    // Open files, rook activity
    "r4rk1/pp2bppp/2n1p3/q2pP3/3P4/2N2N2/PP2BPPP/R2Q1RK1 w - - 0 12",
    // Isolated queen pawn
    "r1bq1rk1/pp3ppp/2nbpn2/3p4/3P4/2NBPN2/PP3PPP/R1BQ1RK1 w - - 0 9",
    // Hanging pawns
    "r1b2rk1/pp2bppp/2n1pn2/q1pp4/2PP4/1PN2NP1/PB2PP1P/R2QKB1R w KQ - 0 9",
    // Sharp Sicilian with opposite-side castling
    "2rqkb1r/pp1bpppp/5n2/3p4/3P1B2/2N2Q2/PPP2PPP/R3KB1R w KQk - 0 9",
    // Pawn storm position
    "r1bq1rk1/pp1n1ppp/4pn2/2ppP3/3P4/P1N2P2/1PP3PP/R1BQKB1R w KQ - 0 9",
    // === Late middlegame (10) -- some pieces traded ===
    // RN vs RB
    "r4rk1/pp2bppp/4pn2/8/3P4/2N5/PPP2PPP/R4RK1 w - - 0 14",
    // Q vs RR
    "6k1/pp3ppp/4p3/8/3P4/8/PPP2PPP/3R1RK1 w - - 0 20",
    // Rook and minor piece endgame approaching
    "r3r1k1/pp2bppp/2n1p3/8/3P4/2N5/PPP1BPPP/R3R1K1 w - - 0 15",
    // Queens traded, imbalanced minors
    "r4rk1/pp1nbppp/4p3/3p4/3P4/2NB4/PPP2PPP/R4RK1 w - - 0 14",
    // One minor per side
    "r4rk1/pp2bppp/4p3/3p4/3P4/4B3/PPP2PPP/R4RK1 w - - 0 16",
    // Rook + pawns each
    "3r2k1/pp3ppp/4p3/3p4/3P4/4P3/PPP2PPP/3R2K1 w - - 0 20",
    // Bishop pair vs knight pair
    "r4rk1/pp2bppp/4pn2/3p4/3P4/2N1BN2/PPP2PPP/R4RK1 w - - 0 14",
    // Transitional with queens
    "2rq1rk1/pp2bppp/4pn2/3p4/3P4/2N1BN2/PPP1QPPP/2R2RK1 w - - 0 14",
    // Light piece endgame approaching
    "8/pp2bpkp/4p1p1/3p4/3P4/4B1P1/PPP2P1P/6K1 w - - 0 22",
    // Rook vs rook with minor piece advantage
    "3r2k1/pp2bppp/4p3/3p4/3P4/4BN2/PPP2PPP/3R2K1 w - - 0 18",
    // === Early endgame (10) -- rook, bishop, knight endgames ===
    // Rook endgame with pawns
    "8/pp3kpp/4p3/3pP3/3P2P1/8/PPP2K1P/8 w - - 0 28",
    // Bishop endgame
    "8/pp3kpp/4p3/3p4/3P4/4B3/PPP2KPP/8 w - - 0 25",
    // Knight endgame
    "8/pp3kpp/4p3/3p4/3P4/4N3/PPP2KPP/8 w - - 0 25",
    // Rook + minor piece endgame
    "3r2k1/pp3ppp/4p3/3p4/3P4/4B3/PPP2PPP/3R2K1 w - - 0 22",
    // Rook vs rook, pawn up
    "3r2k1/pp3ppp/4p3/3pP3/8/8/PPP2PPP/3R2K1 w - - 0 24",
    // Same-colored bishop endgame
    "8/pp2bkpp/4p3/3p4/3P4/2B5/PPP2KPP/8 w - - 0 26",
    // Opposite-colored bishop endgame
    "8/pp3kpp/4pb2/3p4/3P4/2B5/PPP2KPP/8 w - - 0 26",
    // Rook + knight vs rook
    "3r2k1/pp3ppp/8/3p4/3P4/4N3/PPP2PPP/3R2K1 w - - 0 24",
    // Two rooks endgame
    "3rr1k1/pp3ppp/8/3p4/3P4/8/PPP2PPP/3RR1K1 w - - 0 22",
    // Knight vs bishop endgame
    "8/pp3kpp/4p3/3p4/3Pn3/2B5/PPP2KPP/8 w - - 0 28",
    // === Late endgame (10) -- few pieces, king activity critical ===
    // King + pawns vs king + pawns
    "8/pp3k2/8/3p4/3P4/8/PPP2K2/8 w - - 0 35",
    // K+R vs K+R, equal pawns
    "8/pp3k2/8/8/8/8/PPP2K2/8 w - - 0 40",
    // K+Q vs K+R
    "8/5k2/8/8/8/8/5K2/8 w - - 0 45",
    // K+B+N vs K (mating position)
    "8/8/8/8/4k3/8/2BN4/4K3 w - - 0 1",
    // Two connected passed pawns
    "8/8/8/5k2/8/3PP3/5K2/8 w - - 0 40",
    // Pawn race
    "8/p4k2/8/8/8/8/4K2P/8 w - - 0 40",
    // Lucena position
    "1K1k4/1P6/8/8/8/8/r7/2R5 w - - 0 1",
    // Philidor position
    "8/8/8/4k3/4P3/4K3/8/4r3 w - - 0 1",
    // King + 2 pawns vs king
    "8/8/4k3/8/8/8/4PP2/4K3 w - - 0 1",
    // Queen vs pawn on 7th
    "8/3Pk3/8/8/8/8/8/4K2q w - - 0 1",
    // === Pawn structure extremes (5) ===
    // All pawns, all pieces minus queens
    "rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNB1KBNR w KQkq - 0 1",
    // Doubled and isolated pawns
    "4k3/pp1p1p1p/2p3p1/8/8/2P3P1/PP1P1P1P/4K3 w - - 0 1",
    // All passed pawns
    "4k3/8/8/pppppppp/PPPPPPPP/8/8/4K3 w - - 0 1",
    // Single file pawns (extreme doubled)
    "4k3/4p3/4p3/4p3/4P3/4P3/4P3/4K3 w - - 0 1",
    // No pawns at all
    "rnbqkbnr/8/8/8/8/8/8/RNBQKBNR w KQkq - 0 1",
    // === Asymmetric material (5) ===
    // Queen vs two rooks
    "4k3/8/8/8/8/8/8/RR2K2q w Q - 0 1",
    // Rook vs bishop + knight
    "4k3/8/8/8/8/2bn4/8/R3K3 w Q - 0 1",
    // Three minor pieces vs queen
    "4k3/8/8/8/8/1bbn4/8/4K2Q w - - 0 1",
    // Rook + pawn vs two minor pieces
    "4k3/8/8/8/4P3/2bn4/8/R3K3 w - - 0 1",
    // Two bishops vs rook
    "4k3/8/8/8/8/2BB4/8/r3K3 w - - 0 1",
];

fn evaluation_benchmark(c: &mut Criterion) {
    let positions: Vec<Board> = BENCHMARK_FENS
        .iter()
        .map(|fen| Board::from_str(fen).unwrap())
        .collect();

    c.bench_function("eval_suite_all_positions", |b| {
        b.iter(|| {
            let mut total: i16 = 0;
            for board in &positions {
                total = total.wrapping_add(board_material_score(board));
            }
            black_box(total)
        })
    });
}

criterion_group!(benches, evaluation_benchmark);
criterion_main!(benches);
