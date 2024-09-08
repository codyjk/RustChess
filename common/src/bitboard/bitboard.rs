use core::fmt;
use std::{
    fmt::{Display, Formatter},
    ops::{
        Add, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl, ShlAssign,
        Shr, ShrAssign, Sub,
    },
};

use crate::bitboard::square::from_rank_file;

/// Represents a chess board as a 64-bit integer. In practice, there will be
/// one bitboard for each player's piece type (e.g. white pawns, black knights).
/// Additionally, bitboards are used throughout the application to represent
/// occupied squares, attack maps, and individual squares.
/// By representing the board as a single 64-bit integer, we can take advantage
/// of the CPU's bitwise operations to quickly calculate moves, attacks, and other
/// board state changes.
#[derive(Clone, Copy, PartialEq, Debug, PartialOrd, Eq, Ord, Hash)]
pub struct Bitboard(pub u64);

impl Bitboard {
    pub const EMPTY: Self = Self(0x0000000000000000);
    pub const ALL: Self = Self(0xFFFFFFFFFFFFFFFF);

    pub const A_FILE: Self = Self(0x0101010101010101);
    pub const B_FILE: Self = Self(0x0202020202020202);
    pub const C_FILE: Self = Self(0x0404040404040404);
    pub const D_FILE: Self = Self(0x0808080808080808);
    pub const E_FILE: Self = Self(0x1010101010101010);
    pub const F_FILE: Self = Self(0x2020202020202020);
    pub const G_FILE: Self = Self(0x4040404040404040);
    pub const H_FILE: Self = Self(0x8080808080808080);

    pub const RANK_1: Self = Self(0xFF);
    pub const RANK_2: Self = Self(0xFF00);
    pub const RANK_3: Self = Self(0xFF0000);
    pub const RANK_4: Self = Self(0xFF000000);
    pub const RANK_5: Self = Self(0xFF00000000);
    pub const RANK_6: Self = Self(0xFF0000000000);
    pub const RANK_7: Self = Self(0xFF000000000000);
    pub const RANK_8: Self = Self(0xFF00000000000000);

    pub fn overlaps(&self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub fn trailing_zeros(&self) -> u32 {
        self.0.trailing_zeros()
    }

    pub fn count_ones(&self) -> u32 {
        self.0.count_ones()
    }

    pub fn popcnt(&self) -> u32 {
        self.0.count_ones()
    }
}

/// These macros efficiently implement bitwise operations for the Bitboard struct.
/// They generate the necessary trait implementations for various operations,
/// reducing code duplication and improving maintainability.
///
/// Without macros, we would need to manually implement each trait for every
/// operation, leading to repetitive and error-prone code. These macros allow
/// us to define the implementation pattern once and reuse it for multiple
/// operations, ensuring consistency and reducing the chance of errors.
macro_rules! impl_bitwise_op {
    ($trait:ident, $method:ident, $op:tt) => {
        impl $trait for Bitboard {
            type Output = Self;

            fn $method(self, rhs: Self) -> Self {
                Self(self.0 $op rhs.0)
            }
        }
    };
}

/// This macro implements bitwise assignment operations for the Bitboard struct.
/// It generates in-place modifications of the Bitboard, which can be more
/// efficient in certain scenarios by avoiding the creation of new Bitboard instances.
macro_rules! impl_bitwise_assign_op {
    ($trait:ident, $method:ident, $op:tt) => {
        impl $trait for Bitboard {
            fn $method(&mut self, rhs: Self) {
                self.0 $op rhs.0;
            }
        }
    };
}

/// This macro implements shift operations for the Bitboard struct.
/// It allows for efficient bit shifting, which is crucial for many chess-related
/// calculations and move generation algorithms.
macro_rules! impl_shift_op {
    ($trait:ident, $method:ident, $op:tt) => {
        impl $trait<usize> for Bitboard {
            type Output = Self;

            fn $method(self, rhs: usize) -> Self {
                Self(self.0 $op rhs)
            }
        }
    };
}

/// This macro implements shift assignment operations for the Bitboard struct.
/// It generates in-place shift modifications of the Bitboard, which can be more
/// efficient in certain scenarios by avoiding the creation of new Bitboard instances.
/// These operations are crucial for various bitboard manipulations in chess algorithms.
macro_rules! impl_shift_assign_op {
    ($trait:ident, $method:ident, $op:tt) => {
        impl $trait<usize> for Bitboard {
            fn $method(&mut self, rhs: usize) {
                self.0 $op rhs;
            }
        }
    };
}

impl_bitwise_op!(BitAnd, bitand, &);
impl_bitwise_op!(BitOr, bitor, |);
impl_bitwise_op!(BitXor, bitxor, ^);
impl_bitwise_op!(Add, add, +);
impl_bitwise_op!(Sub, sub, -);

impl_bitwise_assign_op!(BitAndAssign, bitand_assign, &=);
impl_bitwise_assign_op!(BitOrAssign, bitor_assign, |=);
impl_bitwise_assign_op!(BitXorAssign, bitxor_assign, ^=);

impl_shift_op!(Shl, shl, <<);
impl_shift_op!(Shr, shr, >>);

impl_shift_assign_op!(ShlAssign, shl_assign, <<=);
impl_shift_assign_op!(ShrAssign, shr_assign, >>=);

impl Not for Bitboard {
    type Output = Self;

    fn not(self) -> Self {
        Self(!self.0)
    }
}

impl Display for Bitboard {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut result = String::new();
        for rank in (0..8).rev() {
            for file in 0..8 {
                let sq = from_rank_file(rank, file);
                let cell = match self.overlaps(sq) {
                    true => 'X',
                    false => '.',
                };
                result.push(cell);
            }
            result.push('\n');
        }
        write!(f, "{}", result)
    }
}
