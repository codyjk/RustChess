//! Evaluation constants: material values, piece-square tables, and positional
//! scoring tables used by the evaluation function.

use common::bitboard::bitboard::Bitboard;

/// Matches ordering of `Piece` enum.
/// [pawn, knight, bishop, rook, queen, king]
pub const MATERIAL_VALUES: [i16; 6] = [100, 320, 330, 500, 900, 20000];

/// Midgame piece-square tables.
/// [pawn, knight, bishop, rook, queen, king]
#[rustfmt::skip]
pub const BONUS_TABLES_MG: [[i16; 64]; 6] = [
    PAWN_BONUSES_MG,
    KNIGHT_BONUSES,
    BISHOP_BONUSES,
    ROOK_BONUSES,
    QUEEN_BONUSES,
    KING_MIDGAME_BONUSES,
];

/// Endgame piece-square tables.
/// [pawn, knight, bishop, rook, queen, king]
#[rustfmt::skip]
pub const BONUS_TABLES_EG: [[i16; 64]; 6] = [
    PAWN_BONUSES_EG,
    KNIGHT_BONUSES,
    BISHOP_BONUSES,
    ROOK_BONUSES,
    QUEEN_BONUSES,
    KING_ENDGAME_BONUSES,
];

#[rustfmt::skip]
pub const SQUARE_TO_WHITE_BONUS_INDEX: [usize; 64] = [
    56, 57, 58, 59, 60, 61, 62, 63,
    48, 49, 50, 51, 52, 53, 54, 55,
    40, 41, 42, 43, 44, 45, 46, 47,
    32, 33, 34, 35, 36, 37, 38, 39,
    24, 25, 26, 27, 28, 29, 30, 31,
    16, 17, 18, 19, 20, 21, 22, 23,
     8,  9, 10, 11, 12, 13, 14, 15,
     0,  1,  2,  3,  4,  5,  6,  7,
];

#[rustfmt::skip]
pub const SQUARE_TO_BLACK_BONUS_INDEX: [usize; 64] = [
     7,  6,  5,  4,  3,  2,  1,  0,
    15, 14, 13, 12, 11, 10,  9,  8,
    23, 22, 21, 20, 19, 18, 17, 16,
    31, 30, 29, 28, 27, 26, 25, 24,
    39, 38, 37, 36, 35, 34, 33, 32,
    47, 46, 45, 44, 43, 42, 41, 40,
    55, 54, 53, 52, 51, 50, 49, 48,
    63, 62, 61, 60, 59, 58, 57, 56,
];

// --- Pawn PST (midgame and endgame) ---

#[rustfmt::skip]
pub const PAWN_BONUSES_MG: [i16; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
    50, 50, 50, 50, 50, 50, 50, 50,
    10, 10, 20, 30, 30, 20, 10, 10,
     5,  5, 10, 25, 25, 10,  5,  5,
     0,  0,  0, 20, 20,  0,  0,  0,
     5, -5,-10,  0,  0,-10, -5,  5,
     5, 10, 10,-20,-20, 10, 10,  5,
     0,  0,  0,  0,  0,  0,  0,  0
];

/// Endgame pawn table: advanced pawns are more valuable (closer to promotion).
#[rustfmt::skip]
pub const PAWN_BONUSES_EG: [i16; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
    70, 70, 70, 70, 70, 70, 70, 70,
    40, 40, 40, 40, 40, 40, 40, 40,
    20, 20, 20, 20, 20, 20, 20, 20,
    10, 10, 10, 10, 10, 10, 10, 10,
     5,  5,  5,  5,  5,  5,  5,  5,
     0,  0,  0,  0,  0,  0,  0,  0,
     0,  0,  0,  0,  0,  0,  0,  0
];

// --- Other piece PSTs (same for mg/eg for now) ---

#[rustfmt::skip]
pub const KNIGHT_BONUSES: [i16; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50,
];

#[rustfmt::skip]
pub const BISHOP_BONUSES: [i16; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5, 10, 10,  5,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -20,-10,-10,-10,-10,-10,-10,-20,
];

#[rustfmt::skip]
pub const ROOK_BONUSES: [i16; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
     5, 10, 10, 10, 10, 10, 10,  5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
     0,  0,  0,  5,  5,  0,  0,  0
];

#[rustfmt::skip]
pub const QUEEN_BONUSES: [i16; 64] = [
    -20,-10,-10, -5, -5,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5,  5,  5,  5,  0,-10,
     -5,  0,  5,  5,  5,  5,  0, -5,
      0,  0,  5,  5,  5,  5,  0, -5,
    -10,  5,  5,  5,  5,  5,  0,-10,
    -10,  0,  5,  0,  0,  0,  0,-10,
    -20,-10,-10, -5, -5,-10,-10,-20
];

#[rustfmt::skip]
pub const KING_MIDGAME_BONUSES: [i16; 64] = [
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -20,-30,-30,-40,-40,-30,-30,-20,
    -10,-20,-20,-20,-20,-20,-20,-10,
     20, 20,  0,  0,  0,  0, 20, 20,
     20, 30, 10,  0,  0, 10, 30, 20
];

#[rustfmt::skip]
pub const KING_ENDGAME_BONUSES: [i16; 64] = [
    -50,-40,-30,-20,-20,-30,-40,-50,
    -30,-20,-10,  0,  0,-10,-20,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-30,  0,  0,  0,  0,-30,-30,
    -50,-30,-30,-30,-30,-30,-30,-50
];

// --- Game phase weights ---

/// Phase contribution per piece type: Knight=1, Bishop=1, Rook=2, Queen=4.
/// Index matches `Piece` enum: [pawn, knight, bishop, rook, queen, king].
pub const PHASE_WEIGHTS: [u8; 6] = [0, 1, 1, 2, 4, 0];

/// Maximum game phase value (all minor pieces + rooks + queens for both sides).
/// 2*(2*1 + 2*1 + 2*2 + 1*4) = 2*12 = 24
pub const MAX_PHASE: u8 = 24;

// --- File and rank masks for positional evaluation ---

pub const FILE_MASKS: [Bitboard; 8] = [
    Bitboard::A_FILE,
    Bitboard::B_FILE,
    Bitboard::C_FILE,
    Bitboard::D_FILE,
    Bitboard::E_FILE,
    Bitboard::F_FILE,
    Bitboard::G_FILE,
    Bitboard::H_FILE,
];

/// Adjacent file masks for each file (files to the left and right).
pub const ADJACENT_FILES: [Bitboard; 8] = [
    // A: only B
    Bitboard::B_FILE,
    // B: A | C
    Bitboard(Bitboard::A_FILE.0 | Bitboard::C_FILE.0),
    // C: B | D
    Bitboard(Bitboard::B_FILE.0 | Bitboard::D_FILE.0),
    // D: C | E
    Bitboard(Bitboard::C_FILE.0 | Bitboard::E_FILE.0),
    // E: D | F
    Bitboard(Bitboard::D_FILE.0 | Bitboard::F_FILE.0),
    // F: E | G
    Bitboard(Bitboard::E_FILE.0 | Bitboard::G_FILE.0),
    // G: F | H
    Bitboard(Bitboard::F_FILE.0 | Bitboard::H_FILE.0),
    // H: only G
    Bitboard::G_FILE,
];

/// Passed pawn bonus by rank (from White's perspective, index 0 = rank 1).
/// Rank 1 and rank 8 are 0 (pawns can't be on rank 1, rank 8 = promotion).
pub const PASSED_PAWN_BONUS_MG: [i16; 8] = [0, 5, 10, 20, 35, 50, 80, 0];
pub const PASSED_PAWN_BONUS_EG: [i16; 8] = [0, 10, 15, 30, 50, 70, 100, 0];

/// Penalty per extra pawn on the same file (doubled pawns).
pub const DOUBLED_PAWN_PENALTY: i16 = 10;

/// Penalty per isolated pawn (no friendly pawns on adjacent files).
pub const ISOLATED_PAWN_PENALTY: i16 = 12;

/// Bishop pair bonus.
pub const BISHOP_PAIR_BONUS_MG: i16 = 30;
pub const BISHOP_PAIR_BONUS_EG: i16 = 50;

/// Rook on open file bonus (no pawns on file) -- midgame/endgame.
pub const ROOK_OPEN_FILE_BONUS_MG: i16 = 20;
pub const ROOK_OPEN_FILE_BONUS_EG: i16 = 10;

/// Rook on semi-open file bonus (no friendly pawns on file) -- midgame/endgame.
pub const ROOK_SEMI_OPEN_FILE_BONUS_MG: i16 = 10;
pub const ROOK_SEMI_OPEN_FILE_BONUS_EG: i16 = 5;

/// Rook on 7th rank bonus -- midgame/endgame.
pub const ROOK_ON_SEVENTH_BONUS_MG: i16 = 15;
pub const ROOK_ON_SEVENTH_BONUS_EG: i16 = 30;

/// King safety: pawn shield bonus per shielding pawn (midgame only).
pub const PAWN_SHIELD_BONUS: i16 = 8;

/// King safety: penalty per open file near king (midgame only).
pub const KING_OPEN_FILE_PENALTY: i16 = 15;
