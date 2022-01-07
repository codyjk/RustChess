use super::bitboard::EMPTY;
use super::piece::{Piece, ALL_PIECES};
use super::BoardError;
use std::hash::Hash;

#[derive(Clone, Copy, PartialEq, Hash)]
pub struct Pieces {
    pawns: u64,
    rooks: u64,
    knights: u64,
    bishops: u64,
    kings: u64,
    queens: u64,
    occupied: u64,
}

impl Pieces {
    pub fn new() -> Self {
        Pieces {
            bishops: EMPTY,
            kings: EMPTY,
            knights: EMPTY,
            pawns: EMPTY,
            queens: EMPTY,
            rooks: EMPTY,

            occupied: EMPTY,
        }
    }

    pub fn locate(self, piece: Piece) -> u64 {
        match piece {
            Piece::Bishop => self.bishops,
            Piece::King => self.kings,
            Piece::Knight => self.knights,
            Piece::Pawn => self.pawns,
            Piece::Queen => self.queens,
            Piece::Rook => self.rooks,
        }
    }

    pub fn get(self, square: u64) -> Option<Piece> {
        if square & self.bishops > 0 {
            return Some(Piece::Bishop);
        } else if square & self.kings > 0 {
            return Some(Piece::King);
        } else if square & self.knights > 0 {
            return Some(Piece::Knight);
        } else if square & self.pawns > 0 {
            return Some(Piece::Pawn);
        } else if square & self.queens > 0 {
            return Some(Piece::Queen);
        } else if square & self.rooks > 0 {
            return Some(Piece::Rook);
        }

        None
    }

    pub fn occupied(self) -> u64 {
        self.occupied
    }

    pub fn is_occupied(self, square: u64) -> bool {
        square & self.occupied > 0
    }

    pub fn put(&mut self, square: u64, piece: Piece) -> Result<(), BoardError> {
        if self.is_occupied(square) {
            return Err(BoardError::SquareOccupied);
        }

        match piece {
            Piece::Bishop => self.bishops |= square,
            Piece::King => self.kings |= square,
            Piece::Knight => self.knights |= square,
            Piece::Pawn => self.pawns |= square,
            Piece::Queen => self.queens |= square,
            Piece::Rook => self.rooks |= square,
        };

        self.occupied |= square;

        Ok(())
    }

    pub fn remove(&mut self, square: u64) -> Option<Piece> {
        let removed = self.get(square);
        let removed_piece = match removed {
            Some(piece) => piece,
            None => return None,
        };

        match removed_piece {
            Piece::Bishop => self.bishops ^= square,
            Piece::King => self.kings ^= square,
            Piece::Knight => self.knights ^= square,
            Piece::Pawn => self.pawns ^= square,
            Piece::Queen => self.queens ^= square,
            Piece::Rook => self.rooks ^= square,
        };

        self.occupied ^= square;

        removed
    }

    pub fn material_value(&self) -> f32 {
        let mut material = 0.;

        for piece in &ALL_PIECES {
            material +=
                f32::from(count_set_bits(self.locate(*piece))) * f32::from(piece.material_value());
        }

        material
    }
}

fn count_set_bits(mut b: u64) -> u8 {
    let mut r = 0;
    while b > 0 {
        r += 1;
        b &= b - 1
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;

    #[test]
    fn test_count_set_bits() {
        assert_eq!(0, count_set_bits(0b0));
        assert_eq!(4, count_set_bits(0b01010101));
        assert_eq!(8, count_set_bits(0b11111111));
        assert_eq!(1, count_set_bits(0b10000000));
        assert_eq!(1, count_set_bits(0b00000001));
    }

    #[test]
    fn test_material_value() {
        let board = Board::starting_position();
        let starting_material =
            board.white.material_value() - f32::from(Piece::King.material_value());
        // 8 * 1 = 8 pawns
        // 1 * 9 = 9 queens
        // 2 * 5 = 10 rooks
        // 2 * 3 = 6 knights
        // 2 * 3 = 6 bishops
        // total = 39
        assert_eq!(39., starting_material);
    }
}
