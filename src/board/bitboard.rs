pub type Bitboard = u64;

pub const EMPTY: Bitboard = 0x0000000000000000;
pub const ALL: Bitboard = 0xFFFFFFFFFFFFFFFF;

pub const A_FILE: Bitboard = 0x0101010101010101;
pub const B_FILE: Bitboard = A_FILE << 1;
pub const C_FILE: Bitboard = A_FILE << 2;
pub const D_FILE: Bitboard = A_FILE << 3;
pub const E_FILE: Bitboard = A_FILE << 4;
pub const F_FILE: Bitboard = A_FILE << 5;
pub const G_FILE: Bitboard = A_FILE << 6;
pub const H_FILE: Bitboard = A_FILE << 7;

pub const RANK_1: Bitboard = 0xFF;
pub const RANK_2: Bitboard = RANK_1 << (8 * 1);
pub const RANK_3: Bitboard = RANK_1 << (8 * 2);
pub const RANK_4: Bitboard = RANK_1 << (8 * 3);
pub const RANK_5: Bitboard = RANK_1 << (8 * 4);
pub const RANK_6: Bitboard = RANK_1 << (8 * 5);
pub const RANK_7: Bitboard = RANK_1 << (8 * 6);
pub const RANK_8: Bitboard = RANK_1 << (8 * 7);
