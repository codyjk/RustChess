use std::fmt;

#[derive(Clone, Copy, PartialEq, Debug, Eq, PartialOrd, Ord)]
pub enum Piece {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl Piece {
    pub fn from_usize(i: usize) -> Self {
        match i {
            0 => Piece::Pawn,
            1 => Piece::Knight,
            2 => Piece::Bishop,
            3 => Piece::Rook,
            4 => Piece::Queen,
            5 => Piece::King,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_piece_to_usize() {
        // [pawn, knight, bishop, rook, queen, king]
        assert_eq!(Piece::Pawn as usize, 0);
        assert_eq!(Piece::Knight as usize, 1);
        assert_eq!(Piece::Bishop as usize, 2);
        assert_eq!(Piece::Rook as usize, 3);
        assert_eq!(Piece::Queen as usize, 4);
        assert_eq!(Piece::King as usize, 5);
    }

    #[test]
    fn test_piece_from_usize() {
        assert_eq!(Piece::from_usize(0), Piece::Pawn);
        assert_eq!(Piece::from_usize(1), Piece::Knight);
        assert_eq!(Piece::from_usize(2), Piece::Bishop);
        assert_eq!(Piece::from_usize(3), Piece::Rook);
        assert_eq!(Piece::from_usize(4), Piece::Queen);
        assert_eq!(Piece::from_usize(5), Piece::King);
    }

    #[test]
    fn test_piece_to_and_from_usize() {
        for i in 0..6 {
            let piece = Piece::from_usize(i);
            assert_eq!(Piece::from_usize(i as usize) as usize, i);
            assert_eq!(piece as usize, i);
        }
    }
}
