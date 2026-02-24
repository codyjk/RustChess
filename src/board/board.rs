//! Chess board state representation.

use std::str::FromStr;

use common::bitboard::{Bitboard, Square};
#[cfg(feature = "instrumentation")]
use tracing::instrument;

use crate::{
    chess_position,
    input_handler::fen::{parse_fen, FenParseError},
};

use super::{
    castle_rights::CastleRights, error::BoardError, fullmove_number::FullmoveNumber,
    halfmove_clock::HalfmoveClock, move_info::MoveInfo, piece_set::PieceSet,
    position_info::PositionInfo, Color, Piece,
};

/// Represents the state of a chess board.
pub struct Board {
    white: PieceSet,
    black: PieceSet,
    turn: Color,
    move_info: MoveInfo,
    position_info: PositionInfo,
}

impl Default for Board {
    fn default() -> Self {
        chess_position! {
            rnbqkbnr
            pppppppp
            ........
            ........
            ........
            ........
            PPPPPPPP
            RNBQKBNR
        }
    }
}

impl Board {
    pub fn new() -> Self {
        Self {
            white: PieceSet::new(),
            black: PieceSet::new(),
            turn: Color::White,
            move_info: MoveInfo::new(),
            position_info: PositionInfo::new(),
        }
    }

    pub fn pieces(&self, color: Color) -> &PieceSet {
        match color {
            Color::White => &self.white,
            Color::Black => &self.black,
        }
    }

    pub fn occupied(&self) -> Bitboard {
        self.white.occupied() | self.black.occupied()
    }

    pub fn is_occupied(&self, square: Bitboard) -> bool {
        !(self.occupied() & square).is_empty()
    }

    pub fn is_square_occupied(&self, square: Square) -> bool {
        square.overlaps(self.occupied())
    }

    #[cfg_attr(feature = "instrumentation", instrument(skip_all))]
    pub fn get(&self, square: Square) -> Option<(Piece, Color)> {
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

    #[must_use = "placing a piece may fail if the square is occupied"]
    pub fn put(&mut self, square: Square, piece: Piece, color: Color) -> Result<(), BoardError> {
        if square.overlaps(self.occupied()) {
            return Err(BoardError::SquareOccupiedBoardPutError);
        }

        let result = match color {
            Color::White => self.white.put(square, piece),
            Color::Black => self.black.put(square, piece),
        };

        if result.is_ok() {
            self.position_info
                .update_zobrist_hash_toggle_piece(square, piece, color);
        }

        result
    }

    #[cfg_attr(feature = "instrumentation", instrument(skip_all))]
    pub fn remove(&mut self, square: Square) -> Option<(Piece, Color)> {
        let (piece, color) = self.get(square)?;
        match color {
            Color::White => self.white.remove(square),
            Color::Black => self.black.remove(square),
        }?;
        self.position_info
            .update_zobrist_hash_toggle_piece(square, piece, color);
        Some((piece, color))
    }

    pub fn turn(&self) -> Color {
        self.turn
    }

    pub fn toggle_turn(&mut self) -> Color {
        self.turn = self.turn.opposite();
        self.position_info.update_zobrist_hash_toggle_turn();
        self.turn
    }

    pub fn set_turn(&mut self, turn: Color) -> Color {
        self.turn = turn;
        turn
    }

    pub fn push_en_passant_target(&mut self, target_square: Option<Square>) -> Option<Square> {
        self.position_info
            .update_zobrist_hash_toggle_en_passant_target(target_square);
        self.move_info.push_en_passant_target(target_square)
    }

    pub fn peek_en_passant_target(&self) -> Option<Square> {
        self.move_info.peek_en_passant_target()
    }

    pub fn pop_en_passant_target(&mut self) -> Option<Square> {
        let target_square = self.move_info.pop_en_passant_target();
        self.position_info
            .update_zobrist_hash_toggle_en_passant_target(target_square);
        target_square
    }

    pub fn preserve_castle_rights(&mut self) -> CastleRights {
        // zobrist does not change
        self.move_info.preserve_castle_rights()
    }

    pub fn peek_castle_rights(&self) -> CastleRights {
        self.move_info.peek_castle_rights()
    }

    #[cfg_attr(feature = "instrumentation", instrument(skip_all))]
    pub fn lose_castle_rights(&mut self, lost_rights: CastleRights) -> CastleRights {
        let (old_rights, new_rights) = self.move_info.lose_castle_rights(lost_rights);
        self.position_info
            .update_zobrist_hash_toggle_castling_rights(old_rights.bits());
        self.position_info
            .update_zobrist_hash_toggle_castling_rights(new_rights.bits());
        new_rights
    }

    #[cfg_attr(feature = "instrumentation", instrument(skip_all))]
    pub fn pop_castle_rights(&mut self) -> CastleRights {
        let (old_rights, new_rights) = self.move_info.pop_castle_rights();
        self.position_info
            .update_zobrist_hash_toggle_castling_rights(old_rights.bits());
        self.position_info
            .update_zobrist_hash_toggle_castling_rights(new_rights.bits());
        new_rights
    }

    pub fn increment_fullmove_clock(&mut self) -> FullmoveNumber {
        self.move_info.increment_fullmove_clock()
    }

    pub fn decrement_fullmove_clock(&mut self) -> FullmoveNumber {
        self.move_info.decrement_fullmove_clock()
    }

    pub fn set_fullmove_clock(&mut self, clock: FullmoveNumber) -> FullmoveNumber {
        self.move_info.set_fullmove_clock(clock)
    }

    pub fn fullmove_clock(&self) -> FullmoveNumber {
        self.move_info.fullmove_clock()
    }

    pub fn push_halfmove_clock(&mut self, clock: HalfmoveClock) -> HalfmoveClock {
        self.move_info.push_halfmove_clock(clock)
    }

    pub fn increment_halfmove_clock(&mut self) -> HalfmoveClock {
        self.move_info.increment_halfmove_clock()
    }

    pub fn reset_halfmove_clock(&mut self) -> HalfmoveClock {
        self.move_info.reset_halfmove_clock()
    }

    pub fn halfmove_clock(&self) -> HalfmoveClock {
        self.move_info.halfmove_clock()
    }

    pub fn pop_halfmove_clock(&mut self) -> HalfmoveClock {
        self.move_info.pop_halfmove_clock()
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

    /// Convert the board position to FEN (Forsyth-Edwards Notation) string
    pub fn to_fen(&self) -> String {
        crate::input_handler::fen_serialize::to_fen(self)
    }
}

impl Clone for Board {
    fn clone(&self) -> Self {
        crate::diagnostics::memory_profiler::MemoryProfiler::record_board_clone();
        Self {
            white: self.white.clone(),
            black: self.black.clone(),
            turn: self.turn,
            move_info: self.move_info.clone(),
            position_info: self.position_info.clone(),
        }
    }
}

impl FromStr for Board {
    type Err = FenParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        parse_fen(input)
    }
}
