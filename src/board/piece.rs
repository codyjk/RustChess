use std::fmt;

#[derive(Clone, Copy, PartialEq, Debug, Eq, PartialOrd, Ord)]
pub enum Piece {
    Pawn = 0b000001,
    Bishop = 0b000010,
    Knight = 0b000100,
    Rook = 0b001000,
    Queen = 0b010000,
    King = 0b100000,
}

pub const ALL_PIECES: [Piece; 6] = [
    Piece::Pawn,
    Piece::Bishop,
    Piece::Knight,
    Piece::Rook,
    Piece::Queen,
    Piece::King,
];

impl Piece {
    pub fn material_value(self) -> u16 {
        match self {
            Piece::Pawn => 100,
            Piece::Knight => 320,
            Piece::Bishop => 330,
            Piece::Rook => 500,
            Piece::Queen => 900,
            Piece::King => 20000,
        }
    }
}

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
