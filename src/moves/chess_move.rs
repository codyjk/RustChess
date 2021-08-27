use crate::board::bitboard::{RANK_3, RANK_6};
use crate::board::color::Color;
use crate::board::piece::Piece;
use crate::board::square;
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChessMove {
    Basic {
        from_square: u64,
        to_square: u64,
        capture: Option<Capture>,
    },
    EnPassant {
        from_square: u64,
        to_square: u64, // this is the en passant target square
    },
}

type Capture = (Piece, Color);

impl ChessMove {
    pub fn basic(from_square: u64, to_square: u64, capture: Option<Capture>) -> Self {
        Self::Basic {
            from_square: from_square,
            to_square: to_square,
            capture: capture,
        }
    }

    pub fn en_passant(from_square: u64, to_square: u64) -> Self {
        Self::EnPassant {
            from_square: from_square,
            to_square: to_square,
        }
    }

    pub fn from_square(self) -> u64 {
        match self {
            Self::Basic {
                from_square,
                to_square: _,
                capture: _,
            } => from_square,
            Self::EnPassant {
                from_square,
                to_square: _,
            } => from_square,
        }
    }

    pub fn to_square(self) -> u64 {
        match self {
            Self::Basic {
                from_square: _,
                to_square,
                capture: _,
            } => to_square,
            Self::EnPassant {
                from_square: _,
                to_square,
            } => to_square,
        }
    }

    pub fn capture(self) -> Option<Capture> {
        match self {
            Self::Basic {
                from_square: _,
                to_square: _,
                capture,
            } => capture,
            Self::EnPassant {
                from_square: _,
                to_square: en_passant_target_square,
            } => {
                // en passant targets will only ever be on the third or sixth ranks
                let capture = if en_passant_target_square & RANK_3 > 0 {
                    (Piece::Pawn, Color::White)
                } else if en_passant_target_square & RANK_6 > 0 {
                    (Piece::Pawn, Color::Black)
                } else {
                    panic!("en_passant_target_square is impossible");
                };

                Some(capture)
            }
        }
    }
}

impl fmt::Display for ChessMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let capture_msg = match self.capture() {
            Some((piece, color)) => format!(" (captures {})", piece.to_fen(color)),
            None => "".to_string(),
        };

        write!(
            f,
            "{}{}{}",
            square::to_algebraic(self.from_square()).to_lowercase(),
            square::to_algebraic(self.to_square()).to_lowercase(),
            capture_msg
        )
    }
}

impl fmt::Debug for ChessMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let capture_msg = match self.capture() {
            Some((piece, color)) => format!(" (captures {})", piece.to_fen(color)),
            None => "".to_string(),
        };

        write!(
            f,
            "{}{}{}",
            square::to_algebraic(self.from_square()).to_lowercase(),
            square::to_algebraic(self.to_square()).to_lowercase(),
            capture_msg
        )
    }
}
