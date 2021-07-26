use super::bitboard::{Bitboard, EMPTY};
use super::piece::Piece;
use super::square::Square;

#[derive(Clone, Copy, PartialEq)]
pub struct Pieces {
    pawns: Bitboard,
    rooks: Bitboard,
    knights: Bitboard,
    bishops: Bitboard,
    kings: Bitboard,
    queens: Bitboard,
    occupied: Bitboard,
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

    pub fn locate(self, piece: Piece) -> Bitboard {
        match piece {
            Piece::Bishop => self.bishops,
            Piece::King => self.kings,
            Piece::Knight => self.knights,
            Piece::Pawn => self.pawns,
            Piece::Queen => self.queens,
            Piece::Rook => self.rooks,
        }
    }

    pub fn get(self, square: Square) -> Option<Piece> {
        let square_bit = square.to_bitboard();

        if square_bit & self.bishops > 0 {
            return Some(Piece::Bishop);
        } else if square_bit & self.kings > 0 {
            return Some(Piece::King);
        } else if square_bit & self.knights > 0 {
            return Some(Piece::Knight);
        } else if square_bit & self.pawns > 0 {
            return Some(Piece::Pawn);
        } else if square_bit & self.queens > 0 {
            return Some(Piece::Queen);
        } else if square_bit & self.rooks > 0 {
            return Some(Piece::Rook);
        }

        None
    }

    pub fn occupied(self) -> Bitboard {
        self.occupied
    }

    pub fn is_occupied(self, square: Square) -> bool {
        square.to_bitboard() & self.occupied > 0
    }

    pub fn put(&mut self, square: Square, piece: Piece) -> Result<(), &'static str> {
        if self.is_occupied(square) {
            return Err("that square already has a piece on it");
        }

        let square_bit = square.to_bitboard();

        match piece {
            Piece::Bishop => self.bishops |= square_bit,
            Piece::King => self.kings |= square_bit,
            Piece::Knight => self.knights |= square_bit,
            Piece::Pawn => self.pawns |= square_bit,
            Piece::Queen => self.queens |= square_bit,
            Piece::Rook => self.rooks |= square_bit,
        };

        self.occupied |= square_bit;

        Ok(())
    }

    pub fn remove(&mut self, square: Square) -> Option<Piece> {
        let removed = self.get(square);
        let removed_piece = match removed {
            Some(piece) => piece,
            None => return None,
        };

        let square_bit = square.to_bitboard();

        match removed_piece {
            Piece::Bishop => self.bishops ^= square_bit,
            Piece::King => self.kings ^= square_bit,
            Piece::Knight => self.knights ^= square_bit,
            Piece::Pawn => self.pawns ^= square_bit,
            Piece::Queen => self.queens ^= square_bit,
            Piece::Rook => self.rooks ^= square_bit,
        };

        self.occupied ^= square_bit;

        removed
    }
}
