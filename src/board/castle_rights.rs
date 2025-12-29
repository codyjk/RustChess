use super::castle_rights_bitmask::CastleRightsBitmask;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CastleRights(u8);

impl CastleRights {
    pub const fn white_kingside() -> Self {
        Self(0b1000)
    }

    pub const fn black_kingside() -> Self {
        Self(0b0100)
    }

    pub const fn white_queenside() -> Self {
        Self(0b0010)
    }

    pub const fn black_queenside() -> Self {
        Self(0b0001)
    }

    pub const fn all() -> Self {
        Self(
            Self::white_kingside().0
                | Self::black_kingside().0
                | Self::white_queenside().0
                | Self::black_queenside().0,
        )
    }

    pub const fn none() -> Self {
        Self(0)
    }

    pub const fn new(bits: u8) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn contains(self, other: CastleRights) -> bool {
        (self.0 & other.0) == other.0
    }

    pub const fn without(self, other: CastleRights) -> CastleRights {
        CastleRights(self.0 & !other.0)
    }

    pub const fn intersection(self, other: CastleRights) -> CastleRights {
        CastleRights(self.0 & other.0)
    }
}

impl From<CastleRightsBitmask> for CastleRights {
    fn from(bits: CastleRightsBitmask) -> Self {
        Self(bits)
    }
}

impl From<CastleRights> for CastleRightsBitmask {
    fn from(rights: CastleRights) -> Self {
        rights.0
    }
}

impl std::ops::BitOr for CastleRights {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for CastleRights {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl std::ops::BitXor for CastleRights {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl std::ops::Not for CastleRights {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}
