use rustc_hash::FxHashMap;

use super::{
    bitboard::EMPTY,
    color::Color,
    piece::Piece,
    zobrist_tables::{
        ZOBRIST_CASTLING_RIGHTS_TABLE, ZOBRIST_EN_PASSANT_TABLE, ZOBRIST_PIECES_TABLE,
    },
};

/// Stores information about state changes related to the current (and previous) positions.
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
            .unwrap();
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
            .unwrap()
    }

    pub fn max_seen_position_count(&self) -> u8 {
        *self.max_seen_position_count_stack.last().unwrap()
    }

    pub fn update_zobrist_hash_toggle_piece(&mut self, square: u64, piece: Piece, color: Color) {
        let square_num = square.trailing_zeros();
        self.current_position_hash ^=
            ZOBRIST_PIECES_TABLE[piece as usize][square_num as usize][color as usize];
    }

    pub fn update_zobrist_hash_toggle_en_passant_target(&mut self, square: u64) {
        if square == EMPTY {
            return;
        }
        let square_num = square.trailing_zeros();
        self.current_position_hash ^= ZOBRIST_EN_PASSANT_TABLE[square_num as usize];
    }

    pub fn update_zobrist_hash_toggle_castling_rights(&mut self, castling_rights: u8) {
        self.current_position_hash ^= ZOBRIST_CASTLING_RIGHTS_TABLE[castling_rights as usize];
    }

    pub fn current_position_hash(&self) -> u64 {
        self.current_position_hash
    }
}

#[cfg(test)]
mod tests {
    use crate::board::square::ORDERED;

    use super::*;

    #[test]
    fn test_zobrist_hashing_piece_placement() {
        let mut position_info = PositionInfo::new();
        let mut hash = 0;
        for i in 0..64 {
            let random_piece = Piece::from_usize(i % 6);
            position_info.update_zobrist_hash_toggle_piece(1 << i, random_piece, Color::White);
            hash ^= ZOBRIST_PIECES_TABLE[random_piece as usize][i][Color::White as usize];
        }
        assert_eq!(position_info.current_position_hash(), hash);
    }

    #[test]
    fn test_zobrist_hashing_en_passant_target() {
        let mut position_info = PositionInfo::new();
        let mut hash = 0;
        // zip with ORDERED to get the correct square for each zobrist number
        let pairs = ZOBRIST_EN_PASSANT_TABLE.iter().zip(ORDERED.iter());
        for (zobrist_num, square) in pairs {
            position_info.update_zobrist_hash_toggle_en_passant_target(*square);
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
}
