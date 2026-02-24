use common::bitboard::Square;
use rustc_hash::FxHashMap;

use super::{color::Color, piece::Piece};

include!(concat!(env!("OUT_DIR"), "/zobrist_table.rs"));

/// Stores information about state changes related to the current (and previous) positions.
/// Holds the logic for incrementally updating the hash of the current position using
/// Zobrist hashing: https://www.chessprogramming.org/Zobrist_Hashing
#[derive(Clone)]
pub struct PositionInfo {
    position_count: FxHashMap<u64, u8>,
    max_seen_position_count_stack: Vec<u8>,
    current_position_hash: u64,
}

impl Default for PositionInfo {
    fn default() -> Self {
        Self {
            position_count: FxHashMap::default(),
            max_seen_position_count_stack: vec![1],
            current_position_hash: 0,
        }
    }
}

impl PositionInfo {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn count_current_position(&mut self) -> u8 {
        self.position_count
            .entry(self.current_position_hash)
            .and_modify(|count| *count += 1)
            .or_insert(1);
        let count = *self
            .position_count
            .get(&self.current_position_hash)
            .expect("position should exist in map after or_insert");
        self.max_seen_position_count_stack.push(count);
        count
    }

    pub fn uncount_current_position(&mut self) -> u8 {
        self.position_count
            .entry(self.current_position_hash)
            .and_modify(|count| *count -= 1);
        self.max_seen_position_count_stack.pop();
        *self
            .position_count
            .get(&self.current_position_hash)
            .expect("position should exist in map after decrement")
    }

    pub fn max_seen_position_count(&self) -> u8 {
        *self
            .max_seen_position_count_stack
            .last()
            .expect("max_seen_position_count_stack should never be empty")
    }

    pub fn update_zobrist_hash_toggle_piece(&mut self, square: Square, piece: Piece, color: Color) {
        let piece_hash =
            ZOBRIST_PIECES_TABLE[piece as usize][square.index() as usize][color as usize];
        self.current_position_hash ^= piece_hash;
    }

    pub fn update_zobrist_hash_toggle_en_passant_target(&mut self, square: Option<Square>) {
        let Some(sq) = square else { return };
        self.current_position_hash ^= ZOBRIST_EN_PASSANT_TABLE[sq.index() as usize];
    }

    pub fn update_zobrist_hash_toggle_castling_rights(&mut self, castling_rights: u8) {
        self.current_position_hash ^= ZOBRIST_CASTLING_RIGHTS_TABLE[castling_rights as usize];
    }

    pub fn update_zobrist_hash_toggle_turn(&mut self) {
        self.current_position_hash ^= ZOBRIST_TURN;
    }

    pub fn current_position_hash(&self) -> u64 {
        self.current_position_hash
    }
}

#[cfg(test)]
mod tests {
    use common::bitboard::{Square, ORDERED_SQUARES};

    use super::*;

    #[test]
    fn test_zobrist_hashing_piece_placement() {
        let mut position_info = PositionInfo::new();
        let mut hash = 0;
        for i in 0..64 {
            let random_piece = Piece::from_usize(i % 6);
            position_info.update_zobrist_hash_toggle_piece(
                Square::new(i as u8),
                random_piece,
                Color::White,
            );
            hash ^= ZOBRIST_PIECES_TABLE[random_piece as usize][i][Color::White as usize];
        }
        assert_eq!(position_info.current_position_hash(), hash);
    }

    #[test]
    fn test_zobrist_hashing_en_passant_target() {
        let mut position_info = PositionInfo::new();
        let mut hash = 0;
        let pairs = ZOBRIST_EN_PASSANT_TABLE.iter().zip(ORDERED_SQUARES.iter());
        for (zobrist_num, square) in pairs {
            position_info.update_zobrist_hash_toggle_en_passant_target(Some(*square));
            hash ^= zobrist_num;
        }
        assert_eq!(position_info.current_position_hash(), hash);
    }

    #[test]
    fn test_zobrist_hashing_castling_rights() {
        let mut position_info = PositionInfo::new();
        let mut hash = 0;
        for (i, zobrist_num) in ZOBRIST_CASTLING_RIGHTS_TABLE.iter().enumerate() {
            position_info.update_zobrist_hash_toggle_castling_rights(i as u8);
            hash ^= zobrist_num;
        }
        assert_eq!(position_info.current_position_hash(), hash);
    }

    #[test]
    fn test_zobrist_turn_toggle_changes_hash() {
        let mut position_info = PositionInfo::new();
        let original_hash = position_info.current_position_hash();

        position_info.update_zobrist_hash_toggle_turn();
        let toggled_hash = position_info.current_position_hash();
        assert_ne!(
            original_hash, toggled_hash,
            "Toggling turn should change the hash"
        );

        position_info.update_zobrist_hash_toggle_turn();
        let restored_hash = position_info.current_position_hash();
        assert_eq!(
            original_hash, restored_hash,
            "Toggling turn twice should restore the hash"
        );
    }

    #[test]
    fn test_zobrist_piece_hashing_reversible() {
        let mut position_info = PositionInfo::new();
        let mut hash = 0;
        for i in 0..64 {
            let random_piece = Piece::from_usize(i % 6);
            position_info.update_zobrist_hash_toggle_piece(
                Square::new(i as u8),
                random_piece,
                Color::White,
            );
            hash ^= ZOBRIST_PIECES_TABLE[random_piece as usize][i][Color::White as usize];
        }
        assert_eq!(position_info.current_position_hash(), hash);
        for i in 0..64 {
            let random_piece = Piece::from_usize(i % 6);
            position_info.update_zobrist_hash_toggle_piece(
                Square::new(i as u8),
                random_piece,
                Color::White,
            );
            hash ^= ZOBRIST_PIECES_TABLE[random_piece as usize][i][Color::White as usize];
        }
        assert_eq!(position_info.current_position_hash(), 0);
    }
}
