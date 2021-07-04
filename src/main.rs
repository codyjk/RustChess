mod board;

use board::board::*;

const STARTING_POSITION_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

fn main() {
    let board = Board::from_fen(STARTING_POSITION_FEN).unwrap();

    println!("Board:\n{}", board.to_ascii())
}
