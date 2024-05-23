use crate::board::color::Color;
use crate::board::piece::Piece;
use crate::board::square;
use std::fmt;

#[macro_export]
macro_rules! chess_move {
    ($from:expr, $to:expr) => {
        ChessMove::new($from, $to, None)
    };
    ($from:expr, $to:expr, $capture:expr) => {
        ChessMove::new($from, $to, Some($capture))
    };
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum ChessOperation {
    Standard, // moves and captures
    EnPassant,
    Promote { to_piece: Piece },
    Castle,
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
            from_square,
            to_square,
            capture,
        }
    }

    pub fn en_passant(from_square: u64, to_square: u64, capture: Capture) -> Self {
        Self {
            op: ChessOperation::EnPassant,
            from_square,
            to_square,
            capture: Some(capture),
        }
    }

    pub fn promote(
        from_square: u64,
        to_square: u64,
        capture: Option<Capture>,
        promote_to_piece: Piece,
    ) -> Self {
        Self {
            op: ChessOperation::Promote {
                to_piece: promote_to_piece,
            },
            from_square,
            to_square,
            capture,
        }
    }

    fn castle(from_square: u64, to_square: u64) -> Self {
        Self {
            op: ChessOperation::Castle,
            // from and to square refers to the king's square. rook is handled in a special way
            from_square,
            to_square,
            capture: None,
        }
    }

    pub fn castle_kingside(color: Color) -> Self {
        match color {
            Color::White => Self::castle(square::E1, square::G1),
            Color::Black => Self::castle(square::E8, square::G8),
        }
    }

    pub fn castle_queenside(color: Color) -> Self {
        match color {
            Color::White => Self::castle(square::E1, square::C1),
            Color::Black => Self::castle(square::E8, square::C8),
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

impl fmt::Display for ChessOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::Standard => "move".to_string(),
            Self::EnPassant => "en passant".to_string(),
            Self::Promote { to_piece } => format!("promote to {}", to_piece),
            Self::Castle => "castle".to_string(),
        };
        write!(f, "{}", msg)
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
            "{} {}{}{}",
            self.op,
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
            "{} {}{}{}",
            self.op,
            square::to_algebraic(self.from_square).to_lowercase(),
            square::to_algebraic(self.to_square).to_lowercase(),
            capture_msg
        )
    }
}
