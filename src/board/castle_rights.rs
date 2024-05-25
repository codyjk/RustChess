pub type CastleRightsBitmask = u8;
pub const WHITE_KINGSIDE_RIGHTS: CastleRightsBitmask = 0b1000;
pub const BLACK_KINGSIDE_RIGHTS: CastleRightsBitmask = 0b0100;
pub const WHITE_QUEENSIDE_RIGHTS: CastleRightsBitmask = 0b0010;
pub const BLACK_QUEENSIDE_RIGHTS: CastleRightsBitmask = 0b0001;
pub const ALL_CASTLE_RIGHTS: CastleRightsBitmask =
    WHITE_KINGSIDE_RIGHTS | BLACK_KINGSIDE_RIGHTS | WHITE_QUEENSIDE_RIGHTS | BLACK_QUEENSIDE_RIGHTS;
