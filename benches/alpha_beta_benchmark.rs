use chess::alpha_beta_searcher::AlphaBetaSearcher;
use chess::board::castle_rights_bitmask::ALL_CASTLE_RIGHTS;
use chess::board::color::Color;
use chess::board::piece::Piece;
use chess::board::Board;
use chess::chess_position;
use chess::evaluate::{self, GameEnding};
use chess::move_generator::MoveGenerator;

use common::bitboard::bitboard::Bitboard;
use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("alpha beta mate in 2", |b| {
        b.iter(find_alpha_beta_mate_in_2)
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

fn find_alpha_beta_mate_in_2() {
    let mut searcher = AlphaBetaSearcher::new(2);
    let mut move_generator = MoveGenerator::new();
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
    board.lose_castle_rights(ALL_CASTLE_RIGHTS);

    let move1 = searcher.search(&mut board, &mut move_generator).unwrap();
    move1.apply(&mut board).unwrap();
    board.toggle_turn();
    let move2 = searcher.search(&mut board, &mut move_generator).unwrap();
    move2.apply(&mut board).unwrap();
    board.toggle_turn();
    let move3 = searcher.search(&mut board, &mut move_generator).unwrap();
    move3.apply(&mut board).unwrap();
    let current_turn = board.toggle_turn();

    matches!(
        evaluate::game_ending(&mut board, &mut move_generator, current_turn),
        Some(GameEnding::Checkmate)
    );
}
