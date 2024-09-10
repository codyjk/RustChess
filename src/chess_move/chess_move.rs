use core::fmt;

use common::bitboard::{bitboard::Bitboard, square::to_algebraic};

use crate::board::{error::BoardError, piece::Piece, Board};

use super::capture::Capture;
use super::castle::CastleChessMove;
use super::en_passant::EnPassantChessMove;
use super::pawn_promotion::PawnPromotionChessMove;
use super::standard::StandardChessMove;

#[derive(Clone, Eq, PartialOrd, Ord)]
pub enum ChessMove {
    Standard(StandardChessMove),
    PawnPromotion(PawnPromotionChessMove),
    EnPassant(EnPassantChessMove),
    Castle(CastleChessMove),
}

impl ChessMove {
    pub fn to_square(&self) -> Bitboard {
        match self {
            ChessMove::Standard(m) => m.to_square(),
            ChessMove::PawnPromotion(m) => m.to_square(),
            ChessMove::EnPassant(m) => m.to_square(),
            ChessMove::Castle(m) => m.to_square(),
        }
    }

    pub fn from_square(&self) -> Bitboard {
        match self {
            ChessMove::Standard(m) => m.from_square(),
            ChessMove::PawnPromotion(m) => m.from_square(),
            ChessMove::EnPassant(m) => m.from_square(),
            ChessMove::Castle(m) => m.from_square(),
        }
    }

    pub fn captures(&self) -> Option<Capture> {
        match self {
            ChessMove::Standard(m) => m.captures(),
            ChessMove::PawnPromotion(m) => m.captures(),
            ChessMove::EnPassant(m) => Some(m.captures()),
            ChessMove::Castle(_m) => None,
        }
    }

    pub fn apply(&self, board: &mut Board) -> Result<(), BoardError> {
        let result = match self {
            ChessMove::Standard(m) => m.apply(board),
            ChessMove::PawnPromotion(m) => m.apply(board),
            ChessMove::EnPassant(m) => m.apply(board),
            ChessMove::Castle(m) => m.apply(board),
        };

        map_ok(result)
    }

    pub fn undo(&self, board: &mut Board) -> Result<(), BoardError> {
        let result = match self {
            ChessMove::Standard(m) => m.undo(board),
            ChessMove::PawnPromotion(m) => m.undo(board),
            ChessMove::EnPassant(m) => m.undo(board),
            ChessMove::Castle(m) => m.undo(board),
        };

        map_ok(result)
    }

    pub fn to_uci(&self) -> String {
        let from = to_algebraic(self.from_square());
        let to = to_algebraic(self.to_square());
        match self {
            ChessMove::PawnPromotion(m) => {
                format!(
                    "{}{}{}",
                    from,
                    to,
                    match m.promote_to_piece() {
                        Piece::Queen => "q",
                        Piece::Rook => "r",
                        Piece::Bishop => "b",
                        Piece::Knight => "n",
                        _ => panic!("Invalid promotion piece"),
                    }
                )
            }
            _ => format!("{}{}", from, to),
        }
    }
}

impl fmt::Display for ChessMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let move_type = match self {
            ChessMove::Standard(_) => "Move",
            ChessMove::PawnPromotion(m) => match m.promote_to_piece() {
                Piece::Queen => "Promote to Queen",
                Piece::Rook => "Promote to Rook",
                Piece::Bishop => "Promote to Bishop",
                Piece::Knight => "Promote to Knight",
                _ => panic!("Invalid promotion piece"),
            },
            ChessMove::EnPassant(_) => "En Passant",
            ChessMove::Castle(_) => "Castle",
        };
        let from_square = to_algebraic(self.from_square());
        let to_square = to_algebraic(self.to_square());
        let capture = match self.captures() {
            Some(capture) => format!(" capturing {}", capture.0),
            None => "".to_string(),
        };
        write!(f, "{} {}{}{}", move_type, from_square, to_square, capture)
    }
}

fn map_ok<T, E>(result: Result<T, E>) -> Result<(), E> {
    result.map(|_| ())
}

impl fmt::Debug for ChessMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format!("{}", self).fmt(f)
    }
}

impl PartialEq for ChessMove {
    fn eq(&self, other: &ChessMove) -> bool {
        match (self, other) {
            (ChessMove::Standard(a), ChessMove::Standard(b)) => a == b,
            (ChessMove::PawnPromotion(a), ChessMove::PawnPromotion(b)) => a == b,
            (ChessMove::EnPassant(a), ChessMove::EnPassant(b)) => a == b,
            (ChessMove::Castle(a), ChessMove::Castle(b)) => a == b,
            _ => false,
        }
    }
}
