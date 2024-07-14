use std::fmt;

use super::color::Color;

/// Represents a chess piece. The order of the pieces is important,
/// as it is used to index into the tables below.
#[derive(Clone, Copy, PartialEq, Debug, Eq, PartialOrd, Ord)]
pub enum Piece {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

/// Used when rendering algebraic notation.
const ALGEBRAIC_PIECE_STRS: [&str; 6] = ["", "N", "B", "R", "Q", "K"];

/// Used when rendering ASCII notation.
const ASCII_PIECE_CHARS: [[char; 2]; 6] = [
    ['p', 'P'],
    ['n', 'N'],
    ['b', 'B'],
    ['r', 'R'],
    ['q', 'Q'],
    ['k', 'K'],
];

/// Used when rendering the Unicode board.
const UNICODE_PIECE_CHARS: [[char; 2]; 6] = [
    ['♙', '♟'],
    ['♘', '♞'],
    ['♗', '♝'],
    ['♖', '♜'],
    ['♕', '♛'],
    ['♔', '♚'],
];

/// Used in `Display` implementations.
const PIECE_NAMES: [&str; 6] = ["pawn", "knight", "bishop", "rook", "queen", "king"];

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

    pub fn to_char(&self, color: Color) -> char {
        ASCII_PIECE_CHARS[*self as usize][color as usize]
    }

    pub fn from_char(c: char) -> Option<(Piece, Color)> {
        match c {
            'b' => Some((Piece::Bishop, Color::Black)),
            'B' => Some((Piece::Bishop, Color::White)),
            'k' => Some((Piece::King, Color::Black)),
            'K' => Some((Piece::King, Color::White)),
            'n' => Some((Piece::Knight, Color::Black)),
            'N' => Some((Piece::Knight, Color::White)),
            'p' => Some((Piece::Pawn, Color::Black)),
            'P' => Some((Piece::Pawn, Color::White)),
            'q' => Some((Piece::Queen, Color::Black)),
            'Q' => Some((Piece::Queen, Color::White)),
            'r' => Some((Piece::Rook, Color::Black)),
            'R' => Some((Piece::Rook, Color::White)),
            _ => None,
        }
    }

    pub fn to_unicode_piece_char(&self, color: Color) -> char {
        UNICODE_PIECE_CHARS[*self as usize][color as usize]
    }

    pub fn to_algebraic_str(&self) -> &str {
        ALGEBRAIC_PIECE_STRS[*self as usize]
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
        write!(f, "{}", PIECE_NAMES[*self as usize])
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
            assert_eq!(Piece::from_usize(i) as usize, i);
            assert_eq!(piece as usize, i);
        }
    }
}
