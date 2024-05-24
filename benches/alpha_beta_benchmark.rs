use chess::board::color::Color;
use chess::board::piece::Piece;
use chess::board::square::*;
use chess::board::{Board, ALL_CASTLE_RIGHTS};
use chess::evaluate::{self, GameEnding};
use chess::move_generation::targets::Targets;
use chess::searcher::Searcher;

use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("alpha beta mate in 2", |b| {
        b.iter(find_alpha_beta_mate_in_2)
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

fn find_alpha_beta_mate_in_2() {
    let mut board = Board::new();
    let mut targets = Targets::new();
    let mut searcher = Searcher::new(2);

    board.put(F2, Piece::Pawn, Color::White).unwrap();
    board.put(G2, Piece::Pawn, Color::White).unwrap();
    board.put(H2, Piece::Pawn, Color::White).unwrap();
    board.put(G1, Piece::King, Color::White).unwrap();
    board.put(A1, Piece::Rook, Color::White).unwrap();
    board.put(E8, Piece::Rook, Color::Black).unwrap();
    board.put(E7, Piece::Queen, Color::Black).unwrap();
    board.put(H8, Piece::King, Color::Black).unwrap();
    board.set_turn(Color::Black);
    board.lose_castle_rights(ALL_CASTLE_RIGHTS);

    let move1 = searcher.search(&mut board, &mut targets).unwrap();
    move1.apply(&mut board).unwrap();
    board.next_turn();
    let move2 = searcher.search(&mut board, &mut targets).unwrap();
    move2.apply(&mut board).unwrap();
    board.next_turn();
    let move3 = searcher.search(&mut board, &mut targets).unwrap();
    move3.apply(&mut board).unwrap();
    let current_turn = board.next_turn();

    matches!(
        evaluate::game_ending(&mut board, &mut targets, current_turn),
        Some(GameEnding::Checkmate)
    );
}
