pub mod bitboard;
pub mod castle_rights;
pub mod color;
pub mod error;
pub mod magic;
pub mod piece;
pub mod piece_set;
pub mod square;

mod display;
mod fen;
mod move_info;
mod position_info;

use color::Color;
use error::BoardError;
use piece::Piece;
use piece_set::PieceSet;
use rustc_hash::FxHasher;
use std::hash::{Hash, Hasher};

use self::{castle_rights::CastleRightsBitmask, move_info::MoveInfo, position_info::PositionInfo};

pub struct Board {
    white: PieceSet,
    black: PieceSet,
    turn: Color,
    move_info: MoveInfo,
    position_info: PositionInfo,
}

impl Default for Board {
    fn default() -> Self {
        let mut board = Board {
            white: PieceSet::new(),
            black: PieceSet::new(),
            turn: Color::White,
            move_info: MoveInfo::new(),
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
        self.occupied() & square != 0
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
        self.move_info.push_en_passant_target(target_square)
    }

    pub fn peek_en_passant_target(&self) -> u64 {
        self.move_info.peek_en_passant_target()
    }

    pub fn pop_en_passant_target(&mut self) -> u64 {
        self.move_info.pop_en_passant_target()
    }

    pub fn preserve_castle_rights(&mut self) -> CastleRightsBitmask {
        self.move_info.preserve_castle_rights()
    }

    pub fn lose_castle_rights(&mut self, lost_rights: CastleRightsBitmask) -> CastleRightsBitmask {
        self.move_info.lose_castle_rights(lost_rights)
    }

    pub fn peek_castle_rights(&self) -> u8 {
        self.move_info.peek_castle_rights()
    }

    pub fn pop_castle_rights(&mut self) -> CastleRightsBitmask {
        self.move_info.pop_castle_rights()
    }

    pub fn increment_fullmove_clock(&mut self) -> u8 {
        self.move_info.increment_fullmove_clock()
    }

    pub fn decrement_fullmove_clock(&mut self) -> u8 {
        self.move_info.decrement_fullmove_clock()
    }

    pub fn set_fullmove_clock(&mut self, clock: u8) -> u8 {
        self.move_info.set_fullmove_clock(clock)
    }

    pub fn fullmove_clock(&self) -> u8 {
        self.move_info.fullmove_clock()
    }

    pub fn push_halfmove_clock(&mut self, clock: u8) -> u8 {
        self.move_info.push_halfmove_clock(clock)
    }

    pub fn increment_halfmove_clock(&mut self) -> u8 {
        self.move_info.increment_halfmove_clock()
    }

    pub fn reset_halfmove_clock(&mut self) -> u8 {
        self.move_info.reset_halfmove_clock()
    }

    pub fn halfmove_clock(&self) -> u8 {
        self.move_info.halfmove_clock()
    }

    pub fn pop_halfmove_clock(&mut self) -> u8 {
        self.move_info.pop_halfmove_clock()
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
