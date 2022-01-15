use chess::board::color::Color;
use chess::board::piece::Piece;
use chess::board::square;
use chess::board::{Board, ALL_CASTLE_RIGHTS};
use chess::game::{Game, GameEnding};

use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("alpha beta mate in 2", |b| {
        b.iter(|| find_alpha_beta_mate_in_2())
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

fn find_alpha_beta_mate_in_2() {
    let mut board = Board::new();
    board.put(square::F2, Piece::Pawn, Color::White).unwrap();
    board.put(square::G2, Piece::Pawn, Color::White).unwrap();
    board.put(square::H2, Piece::Pawn, Color::White).unwrap();
    board.put(square::G1, Piece::King, Color::White).unwrap();
    board.put(square::A1, Piece::Rook, Color::White).unwrap();
    board.put(square::E8, Piece::Rook, Color::Black).unwrap();
    board.put(square::E7, Piece::Queen, Color::Black).unwrap();
    board.put(square::H8, Piece::King, Color::Black).unwrap();
    board.set_turn(Color::Black);
    board.lose_castle_rights(ALL_CASTLE_RIGHTS);

    let mut game = Game::from_board(board);

    game.make_alpha_beta_best_move(2).unwrap();
    game.next_turn();
    game.make_alpha_beta_best_move(1).unwrap();
    game.next_turn();
    game.make_alpha_beta_best_move(0).unwrap();
    game.next_turn();
    let checkmate = match game.check_game_over_for_current_turn() {
        Some(GameEnding::Checkmate) => true,
        _ => false,
    };
    assert!(checkmate);
}
