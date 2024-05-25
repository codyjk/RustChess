pub mod bitboard;
pub mod color;
pub mod error;
pub mod magic;
pub mod piece;
pub mod piece_set;
pub mod square;

mod display;
mod fen;
mod position_info;

use bitboard::EMPTY;
use color::Color;
use error::BoardError;
use piece::Piece;
use piece_set::PieceSet;
use rustc_hash::FxHasher;
use std::hash::{Hash, Hasher};

use self::position_info::PositionInfo;

type CastleRightsBitmask = u8;
pub const WHITE_KINGSIDE_RIGHTS: CastleRightsBitmask = 0b1000;
pub const BLACK_KINGSIDE_RIGHTS: CastleRightsBitmask = 0b0100;
pub const WHITE_QUEENSIDE_RIGHTS: CastleRightsBitmask = 0b0010;
pub const BLACK_QUEENSIDE_RIGHTS: CastleRightsBitmask = 0b0001;
pub const ALL_CASTLE_RIGHTS: CastleRightsBitmask =
    WHITE_KINGSIDE_RIGHTS | BLACK_KINGSIDE_RIGHTS | WHITE_QUEENSIDE_RIGHTS | BLACK_QUEENSIDE_RIGHTS;

pub struct Board {
    white: PieceSet,
    black: PieceSet,
    turn: Color,
    fullmove_clock: u8,
    en_passant_target_stack: Vec<u64>,
    castle_rights_stack: Vec<CastleRightsBitmask>,
    halfmove_clock_stack: Vec<u8>,
    position_info: PositionInfo,
}


impl Default for Board {
    fn default() -> Self {
        let mut board = Board {
            white: PieceSet::new(),
            black: PieceSet::new(),
            turn: Color::White,
            en_passant_target_stack: vec![EMPTY],
            castle_rights_stack: vec![ALL_CASTLE_RIGHTS],
            halfmove_clock_stack: vec![0],
            fullmove_clock: 1,
            position_info: PositionInfo::new(),
        };
        board.update_position_hash();
        board
    }
}

impl Board {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn pieces(&self, color: Color) -> &PieceSet {
        match color {
            Color::White => &self.white,
            Color::Black => &self.black,
        }
    }

    pub fn starting_position() -> Self {
        Self::from_fen(fen::STARTING_POSITION_FEN).unwrap()
    }

    pub fn occupied(&self) -> u64 {
        self.white.occupied() | self.black.occupied()
    }

    pub fn is_occupied(&self, square: u64) -> bool {
        self.get(square).is_some()
    }

    pub fn get(&self, square: u64) -> Option<(Piece, Color)> {
        let color = if self.white.is_occupied(square) {
            Color::White
        } else if self.black.is_occupied(square) {
            Color::Black
        } else {
            return None;
        };

        let maybe_piece = match color {
            Color::White => self.white.get(square),
            Color::Black => self.black.get(square),
        };

        maybe_piece.map(|piece| (piece, color))
    }

    pub fn put(&mut self, square: u64, piece: Piece, color: Color) -> Result<(), BoardError> {
        if self.is_occupied(square) {
            return Err(BoardError::SquareOccupied);
        }

        match color {
            Color::White => self.white.put(square, piece),
            Color::Black => self.black.put(square, piece),
        }
    }

    pub fn remove(&mut self, square: u64) -> Option<(Piece, Color)> {
        self.get(square).map(|(piece, color)| {
            match color {
                Color::White => self.white.remove(square),
                Color::Black => self.black.remove(square),
            };
            (piece, color)
        })
    }

    pub fn turn(&self) -> Color {
        self.turn
    }

    pub fn toggle_turn(&mut self) -> Color {
        self.turn = self.turn.opposite();
        self.turn
    }

    pub fn set_turn(&mut self, turn: Color) -> Color {
        self.turn = turn;
        turn
    }

    pub fn push_en_passant_target(&mut self, target_square: u64) -> u64 {
        self.en_passant_target_stack.push(target_square);
        target_square
    }

    pub fn peek_en_passant_target(&self) -> u64 {
        *self.en_passant_target_stack.last().unwrap()
    }

    pub fn pop_en_passant_target(&mut self) -> u64 {
        self.en_passant_target_stack.pop().unwrap()
    }

    pub fn preserve_castle_rights(&mut self) -> CastleRightsBitmask {
        let rights = self.peek_castle_rights();
        self.castle_rights_stack.push(rights);
        rights
    }

    pub fn lose_castle_rights(&mut self, lost_rights: CastleRightsBitmask) -> CastleRightsBitmask {
        let old_rights = self.peek_castle_rights();
        let new_rights = old_rights ^ (old_rights & lost_rights);
        self.castle_rights_stack.push(new_rights);
        new_rights
    }

    pub fn peek_castle_rights(&self) -> u8 {
        *self.castle_rights_stack.last().unwrap()
    }

    pub fn pop_castle_rights(&mut self) -> CastleRightsBitmask {
        self.castle_rights_stack.pop().unwrap()
    }

    pub fn increment_fullmove_clock(&mut self) -> u8 {
        self.fullmove_clock += 1;
        self.fullmove_clock
    }

    pub fn decrement_fullmove_clock(&mut self) -> u8 {
        self.fullmove_clock -= 1;
        self.fullmove_clock
    }

    pub fn set_fullmove_clock(&mut self, clock: u8) -> u8 {
        self.fullmove_clock = clock;
        clock
    }

    pub fn fullmove_clock(&self) -> u8 {
        self.fullmove_clock
    }

    pub fn push_halfmove_clock(&mut self, clock: u8) -> u8 {
        self.halfmove_clock_stack.push(clock);
        clock
    }

    pub fn increment_halfmove_clock(&mut self) -> u8 {
        let old_clock = self.halfmove_clock_stack.last().unwrap();
        let new_clock = old_clock + 1;
        self.halfmove_clock_stack.push(new_clock);
        new_clock
    }

    pub fn reset_halfmove_clock(&mut self) -> u8 {
        self.halfmove_clock_stack.push(0);
        0
    }

    pub fn halfmove_clock(&self) -> u8 {
        *self.halfmove_clock_stack.last().unwrap()
    }

    pub fn pop_halfmove_clock(&mut self) -> u8 {
        self.halfmove_clock_stack.pop().unwrap()
    }

    pub fn hashable_position_key(&self) -> [u64; 14] {
        let (a1, b1, c1, d1, e1, f1) = self.black.position_tuple();
        let (a2, b2, c2, d2, e2, f2) = self.white.position_tuple();
        let ep = self.peek_en_passant_target();
        let cr = self.peek_castle_rights() as u64;

        [a1, b1, c1, d1, e1, f1, a2, b2, c2, d2, e2, f2, ep, cr]
    }

    // PositionInfo delegation

    pub fn count_current_position(&mut self) -> u8 {
        self.position_info.count_current_position()
    }

    pub fn uncount_current_position(&mut self) -> u8 {
        self.position_info.uncount_current_position()
    }

    pub fn max_seen_position_count(&self) -> u8 {
        self.position_info.max_seen_position_count()
    }

    pub fn current_position_hash(&self) -> u64 {
        self.position_info.current_position_hash()
    }

    pub fn update_position_hash(&mut self) -> u64 {
        // TODO(codyjk): Replace this with Zobrist hashing
        let mut s = FxHasher::default();
        self.hash(&mut s);
        let hash = s.finish();
        self.position_info.update_position_hash(hash)
    }
}

impl Hash for Board {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.hashable_position_key().hash(state);
    }
}
