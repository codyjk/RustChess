use super::bitboard::EMPTY;
use super::piece::Piece;
use super::BoardError;

#[derive(Clone, PartialEq)]
pub struct Pieces {
    pawns: u64,
    rooks: u64,
    knights: u64,
    bishops: u64,
    kings: u64,
    queens: u64,
    occupied: u64,
}

impl Default for Pieces {
    fn default() -> Self {
        Pieces {
            bishops: EMPTY,
            kings: EMPTY,
            knights: EMPTY,
            pawns: EMPTY,
            queens: EMPTY,
            rooks: EMPTY,
            // helper
            occupied: EMPTY,
        }
    }
}

impl Pieces {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn locate(&self, piece: Piece) -> u64 {
        match piece {
            Piece::Bishop => self.bishops,
            Piece::King => self.kings,
            Piece::Knight => self.knights,
            Piece::Pawn => self.pawns,
            Piece::Queen => self.queens,
            Piece::Rook => self.rooks,
        }
    }

    pub fn get(&self, square: u64) -> Option<Piece> {
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

    pub fn occupied(&self) -> u64 {
        self.occupied
    }

    pub fn is_occupied(&self, square: u64) -> bool {
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

    pub fn position_tuple(&self) -> (u64, u64, u64, u64, u64, u64) {
        (
            self.bishops,
            self.kings,
            self.knights,
            self.pawns,
            self.queens,
            self.rooks,
        )
    }
}
