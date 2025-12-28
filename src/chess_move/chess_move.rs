use core::fmt;

use common::bitboard::Square;

use crate::board::{error::BoardError, piece::Piece, Board};

use super::capture::Capture;
use super::castle::CastleChessMove;
use super::chess_move_effect::ChessMoveEffect;
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

macro_rules! delegate_to_variants {
    ($self:ident, $method:ident, $($variant:ident),*) => {
        match $self {
            $(ChessMove::$variant(m) => m.$method(),)*
        }
    };
}

macro_rules! delegate_to_variants_mut {
    ($self:ident, $method:ident, $arg:expr, $($variant:ident),*) => {
        match $self {
            $(ChessMove::$variant(m) => m.$method($arg),)*
        }
    };
}

impl ChessMove {
    pub fn to_square(&self) -> Square {
        delegate_to_variants!(self, to_square, Standard, PawnPromotion, EnPassant, Castle)
    }

    pub fn from_square(&self) -> Square {
        delegate_to_variants!(self, from_square, Standard, PawnPromotion, EnPassant, Castle)
    }

    pub fn captures(&self) -> Option<Capture> {
        match self {
            ChessMove::Standard(m) => m.captures(),
            ChessMove::PawnPromotion(m) => m.captures(),
            ChessMove::EnPassant(m) => Some(m.captures()),
            ChessMove::Castle(_m) => None,
        }
    }

    pub fn effect(&self) -> Option<ChessMoveEffect> {
        delegate_to_variants!(self, effect, Standard, PawnPromotion, EnPassant, Castle)
    }

    pub fn set_effect(&mut self, effect: ChessMoveEffect) -> &Self {
        delegate_to_variants_mut!(self, set_effect, effect, Standard, PawnPromotion, EnPassant, Castle);
        self
    }

    #[must_use = "move application may fail"]
    pub fn apply(&self, board: &mut Board) -> Result<(), BoardError> {
        let result = delegate_to_variants_mut!(self, apply, board, Standard, PawnPromotion, EnPassant, Castle);
        map_ok(result)
    }

    #[must_use = "move undo may fail"]
    pub fn undo(&self, board: &mut Board) -> Result<(), BoardError> {
        let result = delegate_to_variants_mut!(self, undo, board, Standard, PawnPromotion, EnPassant, Castle);
        map_ok(result)
    }

    pub fn to_uci(&self) -> String {
        let from = self.from_square().to_algebraic();
        let to = self.to_square().to_algebraic();
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
        let from_square = self.from_square().to_algebraic();
        let to_square = self.to_square().to_algebraic();
        let capture = match self.captures() {
            Some(capture) => format!(" capturing {}", capture.0),
            None => "".to_string(),
        };
        let check_or_checkmate_msg = match self.effect() {
            Some(ChessMoveEffect::Check) => " (check)",
            Some(ChessMoveEffect::Checkmate) => " (checkmate)",
            Some(ChessMoveEffect::None) | None => "",
        };
        write!(
            f,
            "{} {}{}{}{}",
            move_type, from_square, to_square, capture, check_or_checkmate_msg
        )
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

#[macro_export]
macro_rules! check_move {
    ($chess_move:expr) => {
        $chess_move.set_effect(ChessMoveEffect::Check).clone()
    };
}

#[macro_export]
macro_rules! checkmate_move {
    ($chess_move:expr) => {
        $chess_move.set_effect(ChessMoveEffect::Checkmate).clone()
    };
}
