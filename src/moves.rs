mod bitboard;
mod fen;

use crate::bitboard::color::Color;
use crate::bitboard::square::Square;
use crate::bitboard::Bitboard;

#[derive(Clone, Copy, PartialEq)]
pub struct ChessMove {
    pub from_square: Square,
    pub to_square: Square,
}

pub fn generate(board: Bitboard, color: Color) -> Vec<ChessMove> {
    let _pieces = board.pieces(color);
    vec![]
}
