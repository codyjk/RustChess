use crate::board::color::Color;
use crate::board::piece::Piece;
use crate::board::square;
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChessOperation {
    Standard, // moves and captures
    EnPassant,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChessMove {
    op: ChessOperation,
    from_square: u64,
    to_square: u64,
    capture: Option<Capture>,
}

type Capture = (Piece, Color);

impl ChessMove {
    pub fn new(from_square: u64, to_square: u64, capture: Option<Capture>) -> Self {
        Self {
            op: ChessOperation::Standard,
            from_square: from_square,
            to_square: to_square,
            capture: capture,
        }
    }

    pub fn en_passant(from_square: u64, to_square: u64, capture: Capture) -> Self {
        Self {
            op: ChessOperation::EnPassant,
            from_square: from_square,
            to_square: to_square,
            capture: Some(capture),
        }
    }

    pub fn op(&self) -> ChessOperation {
        self.op
    }

    pub fn from_square(&self) -> u64 {
        self.from_square
    }

    pub fn to_square(&self) -> u64 {
        self.to_square
    }

    pub fn capture(&self) -> Option<Capture> {
        self.capture
    }
}

impl fmt::Display for ChessMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let capture_msg = match self.capture {
            Some((piece, color)) => format!(" (captures {})", piece.to_fen(color)),
            None => "".to_string(),
        };

        write!(
            f,
            "{}{}{}",
            square::to_algebraic(self.from_square).to_lowercase(),
            square::to_algebraic(self.to_square).to_lowercase(),
            capture_msg
        )
    }
}

impl fmt::Debug for ChessMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let capture_msg = match self.capture {
            Some((piece, color)) => format!(" (captures {})", piece.to_fen(color)),
            None => "".to_string(),
        };

        write!(
            f,
            "{}{}{}",
            square::to_algebraic(self.from_square).to_lowercase(),
            square::to_algebraic(self.to_square).to_lowercase(),
            capture_msg
        )
    }
}
