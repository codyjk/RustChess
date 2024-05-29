use common::bitboard::bitboard::Bitboard;

use super::piece::Piece;
use super::BoardError;

#[derive(Clone, PartialEq)]
pub struct PieceSet {
    // [pawns, rooks, knights, bishops, kings, queens]
    bitboards: [Bitboard; 6],
    occupied: Bitboard,
}

impl Default for PieceSet {
    fn default() -> Self {
        PieceSet {
            bitboards: [Bitboard::EMPTY; 6],
            occupied: Bitboard::EMPTY,
        }
    }
}

impl PieceSet {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn locate(&self, piece: Piece) -> Bitboard {
        self.bitboards[piece as usize]
    }

    pub fn get(&self, square: Bitboard) -> Option<Piece> {
        for (i, &bitboard) in self.bitboards.iter().enumerate() {
            if bitboard.overlaps(square) {
                return Some(Piece::from_usize(i));
            }
        }
        None
    }

    pub fn occupied(&self) -> Bitboard {
        self.occupied
    }

    pub fn is_occupied(&self, square: Bitboard) -> bool {
        self.occupied.overlaps(square)
    }

    pub fn put(&mut self, square: Bitboard, piece: Piece) -> Result<(), BoardError> {
        if self.is_occupied(square) {
            return Err(BoardError::SquareOccupied);
        }

        self.bitboards[piece as usize] |= square;
        self.occupied |= square;

        Ok(())
    }

    pub fn remove(&mut self, square: Bitboard) -> Option<Piece> {
        let removed = self.get(square);
        let removed_piece = match removed {
            Some(piece) => piece,
            None => return None,
        };

        self.bitboards[removed_piece as usize] ^= square;
        self.occupied ^= square;

        removed
    }
}
