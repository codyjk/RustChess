use crate::board::piece::Piece;

/// Represents a captured piece in chess.
#[derive(Debug, PartialEq, Clone, Copy, Eq, PartialOrd, Ord)]
pub struct Capture(pub Piece);
