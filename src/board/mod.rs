pub mod castle_rights_bitmask;
pub mod color;
pub mod error;
pub mod piece;

mod display;
mod move_info;
mod piece_set;
mod position_info;

use color::Color;
use common::bitboard::bitboard::Bitboard;
use error::BoardError;
use piece::Piece;
use piece_set::PieceSet;

use crate::chess_position;

use self::{
    castle_rights_bitmask::CastleRightsBitmask, move_info::MoveInfo, position_info::PositionInfo,
};

/// Represents the state of a chess board. The top level struct holds piece position
/// info, whereas the lower level `move_info` and `position_info` structs hold state
/// related to en passant targets, castling rights, and zobrist hashing.
#[derive(Clone)]
pub struct Board {
    white: PieceSet,
    black: PieceSet,
    turn: Color,
    move_info: MoveInfo,
    position_info: PositionInfo,
}

impl Default for Board {
    fn default() -> Self {
        Self {
            white: PieceSet::new(),
            black: PieceSet::new(),
            turn: Color::White,
            move_info: MoveInfo::new(),
            position_info: PositionInfo::new(),
        }
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

    pub fn occupied(&self) -> Bitboard {
        self.white.occupied() | self.black.occupied()
    }

    pub fn is_occupied(&self, square: Bitboard) -> bool {
        !(self.occupied() & square).is_empty()
    }

    pub fn get(&self, square: Bitboard) -> Option<(Piece, Color)> {
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

    pub fn put(&mut self, square: Bitboard, piece: Piece, color: Color) -> Result<(), BoardError> {
        if self.is_occupied(square) {
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

    pub fn remove(&mut self, square: Bitboard) -> Option<(Piece, Color)> {
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
        self.turn
    }

    pub fn set_turn(&mut self, turn: Color) -> Color {
        self.turn = turn;
        turn
    }

    pub fn push_en_passant_target(&mut self, target_square: Bitboard) -> Bitboard {
        self.position_info
            .update_zobrist_hash_toggle_en_passant_target(target_square);
        self.move_info.push_en_passant_target(target_square)
    }

    pub fn peek_en_passant_target(&self) -> Bitboard {
        self.move_info.peek_en_passant_target()
    }

    pub fn pop_en_passant_target(&mut self) -> Bitboard {
        let target_square = self.move_info.pop_en_passant_target();
        self.position_info
            .update_zobrist_hash_toggle_en_passant_target(target_square);
        target_square
    }

    pub fn preserve_castle_rights(&mut self) -> CastleRightsBitmask {
        // zobrist does not change
        self.move_info.preserve_castle_rights()
    }

    pub fn peek_castle_rights(&self) -> u8 {
        self.move_info.peek_castle_rights()
    }

    pub fn lose_castle_rights(&mut self, lost_rights: CastleRightsBitmask) -> CastleRightsBitmask {
        let (old_rights, new_rights) = self.move_info.lose_castle_rights(lost_rights);
        self.position_info
            .update_zobrist_hash_toggle_castling_rights(old_rights);
        self.position_info
            .update_zobrist_hash_toggle_castling_rights(new_rights);
        new_rights
    }

    pub fn pop_castle_rights(&mut self) -> CastleRightsBitmask {
        let (old_rights, new_rights) = self.move_info.pop_castle_rights();
        self.position_info
            .update_zobrist_hash_toggle_castling_rights(old_rights);
        self.position_info
            .update_zobrist_hash_toggle_castling_rights(new_rights);
        new_rights
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
}

#[cfg(test)]
mod tests {
    use crate::{castle_kingside, std_move};

    use super::*;
    use crate::chess_move::castle::CastleChessMove;
    use crate::chess_move::chess_move::ChessMove;
    use crate::chess_move::standard::StandardChessMove;
    use common::bitboard::square::*;

    #[test]
    fn test_zobrist_hashing_is_equal_for_transpositions() {
        let mut board1 = Board::starting_position();
        let mut board2 = Board::starting_position();
        let initial_hash_1 = board1.current_position_hash();
        let initial_hash_2 = board2.current_position_hash();
        assert_eq!(initial_hash_1, initial_hash_2);

        let board1_moves = vec![
            std_move!(E2, E4),
            std_move!(E7, E5),
            std_move!(G1, F3),
            std_move!(B8, C6),
            std_move!(F1, C4),
            std_move!(G8, F6),
            castle_kingside!(Color::White),
        ];

        let board2_moves = vec![
            std_move!(G1, F3),
            std_move!(B8, C6),
            std_move!(E2, E4),
            std_move!(E7, E5),
            std_move!(F1, C4),
            std_move!(G8, F6),
            castle_kingside!(Color::White),
        ];

        let mut board1_hashes = vec![initial_hash_1];
        let mut board2_hashes = vec![initial_hash_2];

        for (move1, move2) in board1_moves.iter().zip(board2_moves.iter()) {
            move1.apply(&mut board1).unwrap();
            move2.apply(&mut board2).unwrap();
            board1_hashes.push(board1.current_position_hash());
            board2_hashes.push(board2.current_position_hash());
        }
        assert_eq!(
            board1.current_position_hash(),
            board2.current_position_hash()
        );

        // undo the moves and see that we get back to the same position
        board1_hashes.pop();
        board2_hashes.pop();
        for (move1, move2) in board1_moves.iter().rev().zip(board2_moves.iter().rev()) {
            println!("undoing moves {} and {}", move1, move2);
            move1.undo(&mut board1).unwrap();
            move2.undo(&mut board2).unwrap();
            println!(
                "hashes: {} and {}",
                board1.current_position_hash(),
                board2.current_position_hash()
            );
            // compare to the last hash in the vec
            assert_eq!(
                board1.current_position_hash(),
                board1_hashes.pop().unwrap(),
                "hash 1 should be equal after undoing moves"
            );
            assert_eq!(
                board2.current_position_hash(),
                board2_hashes.pop().unwrap(),
                "hash 2 should be equal after undoing moves"
            );
        }
        assert_eq!(
            board1.current_position_hash(),
            board2.current_position_hash(),
            "hashes should be equal after undoing moves"
        );
        assert_eq!(
            initial_hash_1,
            board1.current_position_hash(),
            "hashes should be equal to the initial hash"
        );
        assert_eq!(
            initial_hash_2,
            board2.current_position_hash(),
            "hashes should be equal to the initial hash"
        );
    }
}
