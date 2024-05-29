use core::fmt;
use std::{
    fmt::{Display, Formatter},
    ops::{
        Add, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl, ShlAssign,
        Shr, ShrAssign, Sub,
    },
};

use crate::bitboard::square::from_rank_file;

#[derive(Clone, Copy, PartialEq, Debug, PartialOrd, Eq, Ord, Hash)]
pub struct Bitboard(pub u64);

impl BitAnd for Bitboard {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

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
}

// TODO(codyjk): Maybe generate these with a macro?

impl Not for Bitboard {
    type Output = Self;

    fn not(self) -> Self {
        Self(!self.0)
    }
}

impl BitOr for Bitboard {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitXor for Bitboard {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self {
        Self(self.0 ^ rhs.0)
    }
}

impl Shl<usize> for Bitboard {
    type Output = Self;

    fn shl(self, rhs: usize) -> Self {
        Self(self.0 << rhs)
    }
}

impl Shr<usize> for Bitboard {
    type Output = Self;

    fn shr(self, rhs: usize) -> Self {
        Self(self.0 >> rhs)
    }
}

impl Add for Bitboard {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Bitboard {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl BitAndAssign for Bitboard {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitOrAssign for Bitboard {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitXorAssign for Bitboard {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl ShlAssign<usize> for Bitboard {
    fn shl_assign(&mut self, rhs: usize) {
        self.0 <<= rhs;
    }
}

impl ShrAssign<usize> for Bitboard {
    fn shr_assign(&mut self, rhs: usize) {
        self.0 >>= rhs;
    }
}

impl Display for Bitboard {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut result = String::new();
        for rank in (0..8).rev() {
            for file in 0..8 {
                let sq = from_rank_file(rank, file);
                let cell = match self.overlaps(sq) {
                    true => ' ',
                    false => 'X',
                };
                result.push(cell);
            }
            result.push('\n');
        }
        write!(f, "{}", result)
    }
}
