use common::bitboard::{Bitboard, Square};

use super::piece::Piece;
use super::BoardError;

/// Encapsulates the state for a set of pieces on the board, represented as bitboards.
#[derive(Clone, PartialEq)]
pub struct PieceSet {
    /// Bitboards for each piece type.
    /// [pawns, rooks, knights, bishops, kings, queens]
    bitboards: [Bitboard; 6],

    /// Bitboard representing all occupied squares. Incrementally updated as pieces are added or removed.
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

    pub fn get(&self, square: Square) -> Option<Piece> {
        for (i, &bitboard) in self.bitboards.iter().enumerate() {
            if square.overlaps(bitboard) {
                return Some(Piece::from_usize(i));
            }
        }
        None
    }

    pub fn occupied(&self) -> Bitboard {
        self.occupied
    }

    pub fn is_occupied(&self, square: Square) -> bool {
        square.overlaps(self.occupied)
    }

    pub fn put(&mut self, square: Square, piece: Piece) -> Result<(), BoardError> {
        if square.overlaps(self.occupied) {
            return Err(BoardError::SquareOccupiedBoardPutError);
        }

        let bb = square.to_bitboard();
        self.bitboards[piece as usize] |= bb;
        self.occupied |= bb;

        Ok(())
    }

    pub fn remove(&mut self, square: Square) -> Option<Piece> {
        let removed_piece = self.get(square)?;
        let bb = square.to_bitboard();
        self.bitboards[removed_piece as usize] ^= bb;
        self.occupied ^= bb;
        Some(removed_piece)
    }
}
