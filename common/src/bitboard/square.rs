use once_cell::sync::Lazy;
use regex::Regex;

use crate::bitboard::bitboard::Bitboard;

/// Represents a single square on the chess board (0-63).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Square(u8);

impl Square {
    #[inline]
    pub const fn new(index: u8) -> Self {
        Self(index)
    }

    #[inline]
    pub const fn index(self) -> u8 {
        self.0
    }

    #[inline]
    pub const fn rank(self) -> u8 {
        self.0 / 8
    }

    #[inline]
    pub const fn file(self) -> u8 {
        self.0 % 8
    }

    #[inline]
    pub const fn to_bitboard(self) -> Bitboard {
        Bitboard(1 << self.0)
    }

    #[inline]
    pub const fn from_rank_file(rank: u8, file: u8) -> Self {
        Self(file + rank * 8)
    }

    pub fn to_algebraic(self) -> &'static str {
        ALGEBRAIC[self.0 as usize]
    }

    /// Returns true if this square overlaps with the given bitboard.
    #[inline]
    pub fn overlaps(self, bitboard: Bitboard) -> bool {
        self.to_bitboard().overlaps(bitboard)
    }

    pub fn from_algebraic(s: &str) -> Option<Self> {
        static RE: Lazy<Regex> = Lazy::new(|| Regex::new("^([a-hA-H])([1-8])$").unwrap());
        let caps = RE.captures(s)?;
        let file = match caps[1].chars().next()?.to_ascii_lowercase() {
            'a' => 0,
            'b' => 1,
            'c' => 2,
            'd' => 3,
            'e' => 4,
            'f' => 5,
            'g' => 6,
            'h' => 7,
            _ => return None,
        };
        let rank = caps[2].chars().next()?.to_digit(10)? as u8 - 1;
        Some(Self::from_rank_file(rank, file))
    }
}

impl std::fmt::Debug for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_algebraic())
    }
}

impl std::fmt::Display for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_algebraic())
    }
}

impl From<Square> for Bitboard {
    #[inline]
    fn from(sq: Square) -> Bitboard {
        sq.to_bitboard()
    }
}

impl std::ops::BitOr<Square> for Bitboard {
    type Output = Bitboard;

    #[inline]
    fn bitor(self, rhs: Square) -> Bitboard {
        self | rhs.to_bitboard()
    }
}

impl std::ops::BitOr<Square> for Square {
    type Output = Bitboard;

    #[inline]
    fn bitor(self, rhs: Square) -> Bitboard {
        self.to_bitboard() | rhs.to_bitboard()
    }
}

impl std::ops::BitXor<Square> for Bitboard {
    type Output = Bitboard;

    #[inline]
    fn bitxor(self, rhs: Square) -> Bitboard {
        self ^ rhs.to_bitboard()
    }
}

// Square constants
impl Square {
    pub const A1: Square = Square(0);
    pub const B1: Square = Square(1);
    pub const C1: Square = Square(2);
    pub const D1: Square = Square(3);
    pub const E1: Square = Square(4);
    pub const F1: Square = Square(5);
    pub const G1: Square = Square(6);
    pub const H1: Square = Square(7);
    pub const A2: Square = Square(8);
    pub const B2: Square = Square(9);
    pub const C2: Square = Square(10);
    pub const D2: Square = Square(11);
    pub const E2: Square = Square(12);
    pub const F2: Square = Square(13);
    pub const G2: Square = Square(14);
    pub const H2: Square = Square(15);
    pub const A3: Square = Square(16);
    pub const B3: Square = Square(17);
    pub const C3: Square = Square(18);
    pub const D3: Square = Square(19);
    pub const E3: Square = Square(20);
    pub const F3: Square = Square(21);
    pub const G3: Square = Square(22);
    pub const H3: Square = Square(23);
    pub const A4: Square = Square(24);
    pub const B4: Square = Square(25);
    pub const C4: Square = Square(26);
    pub const D4: Square = Square(27);
    pub const E4: Square = Square(28);
    pub const F4: Square = Square(29);
    pub const G4: Square = Square(30);
    pub const H4: Square = Square(31);
    pub const A5: Square = Square(32);
    pub const B5: Square = Square(33);
    pub const C5: Square = Square(34);
    pub const D5: Square = Square(35);
    pub const E5: Square = Square(36);
    pub const F5: Square = Square(37);
    pub const G5: Square = Square(38);
    pub const H5: Square = Square(39);
    pub const A6: Square = Square(40);
    pub const B6: Square = Square(41);
    pub const C6: Square = Square(42);
    pub const D6: Square = Square(43);
    pub const E6: Square = Square(44);
    pub const F6: Square = Square(45);
    pub const G6: Square = Square(46);
    pub const H6: Square = Square(47);
    pub const A7: Square = Square(48);
    pub const B7: Square = Square(49);
    pub const C7: Square = Square(50);
    pub const D7: Square = Square(51);
    pub const E7: Square = Square(52);
    pub const F7: Square = Square(53);
    pub const G7: Square = Square(54);
    pub const H7: Square = Square(55);
    pub const A8: Square = Square(56);
    pub const B8: Square = Square(57);
    pub const C8: Square = Square(58);
    pub const D8: Square = Square(59);
    pub const E8: Square = Square(60);
    pub const F8: Square = Square(61);
    pub const G8: Square = Square(62);
    pub const H8: Square = Square(63);

    pub const ALL: [Square; 64] = [
        Self::A1, Self::B1, Self::C1, Self::D1, Self::E1, Self::F1, Self::G1, Self::H1,
        Self::A2, Self::B2, Self::C2, Self::D2, Self::E2, Self::F2, Self::G2, Self::H2,
        Self::A3, Self::B3, Self::C3, Self::D3, Self::E3, Self::F3, Self::G3, Self::H3,
        Self::A4, Self::B4, Self::C4, Self::D4, Self::E4, Self::F4, Self::G4, Self::H4,
        Self::A5, Self::B5, Self::C5, Self::D5, Self::E5, Self::F5, Self::G5, Self::H5,
        Self::A6, Self::B6, Self::C6, Self::D6, Self::E6, Self::F6, Self::G6, Self::H6,
        Self::A7, Self::B7, Self::C7, Self::D7, Self::E7, Self::F7, Self::G7, Self::H7,
        Self::A8, Self::B8, Self::C8, Self::D8, Self::E8, Self::F8, Self::G8, Self::H8,
    ];
}

// Module-level constants for backward compatibility
pub const A1: Square = Square::A1;
pub const B1: Square = Square::B1;
pub const C1: Square = Square::C1;
pub const D1: Square = Square::D1;
pub const E1: Square = Square::E1;
pub const F1: Square = Square::F1;
pub const G1: Square = Square::G1;
pub const H1: Square = Square::H1;
pub const A2: Square = Square::A2;
pub const B2: Square = Square::B2;
pub const C2: Square = Square::C2;
pub const D2: Square = Square::D2;
pub const E2: Square = Square::E2;
pub const F2: Square = Square::F2;
pub const G2: Square = Square::G2;
pub const H2: Square = Square::H2;
pub const A3: Square = Square::A3;
pub const B3: Square = Square::B3;
pub const C3: Square = Square::C3;
pub const D3: Square = Square::D3;
pub const E3: Square = Square::E3;
pub const F3: Square = Square::F3;
pub const G3: Square = Square::G3;
pub const H3: Square = Square::H3;
pub const A4: Square = Square::A4;
pub const B4: Square = Square::B4;
pub const C4: Square = Square::C4;
pub const D4: Square = Square::D4;
pub const E4: Square = Square::E4;
pub const F4: Square = Square::F4;
pub const G4: Square = Square::G4;
pub const H4: Square = Square::H4;
pub const A5: Square = Square::A5;
pub const B5: Square = Square::B5;
pub const C5: Square = Square::C5;
pub const D5: Square = Square::D5;
pub const E5: Square = Square::E5;
pub const F5: Square = Square::F5;
pub const G5: Square = Square::G5;
pub const H5: Square = Square::H5;
pub const A6: Square = Square::A6;
pub const B6: Square = Square::B6;
pub const C6: Square = Square::C6;
pub const D6: Square = Square::D6;
pub const E6: Square = Square::E6;
pub const F6: Square = Square::F6;
pub const G6: Square = Square::G6;
pub const H6: Square = Square::H6;
pub const A7: Square = Square::A7;
pub const B7: Square = Square::B7;
pub const C7: Square = Square::C7;
pub const D7: Square = Square::D7;
pub const E7: Square = Square::E7;
pub const F7: Square = Square::F7;
pub const G7: Square = Square::G7;
pub const H7: Square = Square::H7;
pub const A8: Square = Square::A8;
pub const B8: Square = Square::B8;
pub const C8: Square = Square::C8;
pub const D8: Square = Square::D8;
pub const E8: Square = Square::E8;
pub const F8: Square = Square::F8;
pub const G8: Square = Square::G8;
pub const H8: Square = Square::H8;

#[rustfmt::skip]
pub const ORDERED_SQUARES: [Square; 64] = [
    A1, A2, A3, A4, A5, A6, A7, A8,
    B1, B2, B3, B4, B5, B6, B7, B8,
    C1, C2, C3, C4, C5, C6, C7, C8,
    D1, D2, D3, D4, D5, D6, D7, D8,
    E1, E2, E3, E4, E5, E6, E7, E8,
    F1, F2, F3, F4, F5, F6, F7, F8,
    G1, G2, G3, G4, G5, G6, G7, G8,
    H1, H2, H3, H4, H5, H6, H7, H8,
];

#[rustfmt::skip]
const ALGEBRAIC: [&str; 64] = [
    "a1", "b1", "c1", "d1", "e1", "f1", "g1", "h1",
    "a2", "b2", "c2", "d2", "e2", "f2", "g2", "h2",
    "a3", "b3", "c3", "d3", "e3", "f3", "g3", "h3",
    "a4", "b4", "c4", "d4", "e4", "f4", "g4", "h4",
    "a5", "b5", "c5", "d5", "e5", "f5", "g5", "h5",
    "a6", "b6", "c6", "d6", "e6", "f6", "g6", "h6",
    "a7", "b7", "c7", "d7", "e7", "f7", "g7", "h7",
    "a8", "b8", "c8", "d8", "e8", "f8", "g8", "h8",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_square_from_rank_file() {
        assert_eq!(Square::A1, Square::from_rank_file(0, 0));
        assert_eq!(Square::B2, Square::from_rank_file(1, 1));
        assert_eq!(Square::E4, Square::from_rank_file(3, 4));
    }

    #[test]
    fn test_square_from_algebraic() {
        assert_eq!(Some(Square::A1), Square::from_algebraic("A1"));
        assert_eq!(Some(Square::A1), Square::from_algebraic("a1"));
        assert_eq!(Some(Square::E5), Square::from_algebraic("E5"));
        assert_eq!(None, Square::from_algebraic("invalid"));
    }

    #[test]
    fn test_square_to_algebraic() {
        assert_eq!("a1", Square::A1.to_algebraic());
        assert_eq!("a8", Square::A8.to_algebraic());
        assert_eq!("h8", Square::H8.to_algebraic());
    }

    #[test]
    fn test_square_to_bitboard() {
        assert_eq!(Bitboard(1), Square::A1.to_bitboard());
        assert_eq!(Bitboard(1 << 63), Square::H8.to_bitboard());
    }
}
