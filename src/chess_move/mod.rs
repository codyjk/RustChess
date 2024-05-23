use core::fmt;

use crate::board::{
    color::Color,
    error::BoardError,
    piece::Piece,
    square::to_algebraic,
    Board,
};

pub mod castle;
pub mod chess_move_collection;
pub mod en_passant;
pub mod pawn_promotion;
pub mod standard;

type Capture = (Piece, Color);

pub trait ChessMove {
    fn to_square(&self) -> u64;
    fn from_square(&self) -> u64;
    fn capture(&self) -> Option<Capture>;
    fn apply(&self, board: &mut Board) -> Result<Option<Capture>, BoardError>;
    fn undo(&self, board: &mut Board) -> Result<Option<Capture>, BoardError>;
}

impl fmt::Display for dyn ChessMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let from_square = to_algebraic(self.from_square());
        let to_square = to_algebraic(self.to_square());
        let capture = match self.capture() {
            Some((piece, _)) => format!(" capturing {}", piece),
            None => "".to_string(),
        };
        write!(f, "{}{}{}", from_square, to_square, capture)
    }
}

impl fmt::Debug for dyn ChessMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let from_square = to_algebraic(self.from_square());
        let to_square = to_algebraic(self.to_square());
        let capture = match self.capture() {
            Some((piece, _)) => format!(" capturing {}", piece),
            None => "".to_string(),
        };
        write!(f, "{}{}{}", from_square, to_square, capture)
    }
}

impl PartialEq for Box<dyn ChessMove> {
    fn eq(&self, other: &Box<dyn ChessMove>) -> bool {
        self.to_square() == other.to_square()
            && self.from_square() == other.from_square()
            && self.capture() == other.capture()
    }
}
