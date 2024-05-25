use std::fmt;

#[derive(Clone, Copy, PartialEq, Debug, Eq, PartialOrd, Ord)]
pub enum Piece {
    Bishop,
    King,
    Knight,
    Pawn,
    Queen,
    Rook,
}

impl Piece {
    pub fn from_usize(i: usize) -> Self {
        match i {
            0 => Self::Bishop,
            1 => Self::King,
            2 => Self::Knight,
            3 => Self::Pawn,
            4 => Self::Queen,
            5 => Self::Rook,
            _ => panic!("Invalid piece index"),
        }
    }
}

pub const ALL_PIECES: [Piece; 6] = [
    Piece::Bishop,
    Piece::King,
    Piece::Knight,
    Piece::Pawn,
    Piece::Queen,
    Piece::Rook,
];

impl fmt::Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::Bishop => "bishop",
            Self::King => "king",
            Self::Knight => "knight",
            Self::Pawn => "pawn",
            Self::Queen => "queen",
            Self::Rook => "rook",
        };
        write!(f, "{}", msg)
    }
}
