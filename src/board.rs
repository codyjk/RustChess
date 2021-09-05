pub mod bitboard;
pub mod color;
pub mod piece;
pub mod square;

mod fen;
mod pieces;
mod ui;

use bitboard::EMPTY;
use color::Color;
use piece::Piece;
use pieces::Pieces;

type CastleRightsBitmask = u8;
pub const WHITE_KINGSIDE_RIGHTS: CastleRightsBitmask = 0b1000;
pub const BLACK_KINGSIDE_RIGHTS: CastleRightsBitmask = 0b0100;
pub const WHITE_QUEENSIDE_RIGHTS: CastleRightsBitmask = 0b0010;
pub const BLACK_QUEENSIDE_RIGHTS: CastleRightsBitmask = 0b0001;

pub struct Board {
    white: Pieces,
    black: Pieces,
    turn: Color,
    en_passant_target_stack: Vec<u64>,
    castle_rights_stack: Vec<CastleRightsBitmask>,
}

impl Board {
    pub fn new() -> Self {
        Board {
            white: Pieces::new(),
            black: Pieces::new(),
            turn: Color::White,
            en_passant_target_stack: vec![EMPTY],
            castle_rights_stack: vec![
                WHITE_KINGSIDE_RIGHTS
                    | BLACK_KINGSIDE_RIGHTS
                    | WHITE_QUEENSIDE_RIGHTS
                    | BLACK_QUEENSIDE_RIGHTS,
            ],
        }
    }

    pub fn pieces(&self, color: Color) -> Pieces {
        match color {
            Color::White => self.white,
            Color::Black => self.black,
        }
    }

    pub fn starting_position() -> Self {
        Self::from_fen(fen::STARTING_POSITION_FEN).unwrap()
    }

    pub fn occupied(&self) -> u64 {
        self.white.occupied() | self.black.occupied()
    }

    pub fn is_occupied(&self, square: u64) -> bool {
        self.get(square).is_some()
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

        match maybe_piece {
            Some(piece) => Some((piece, color)),
            None => None,
        }
    }

    pub fn put(&mut self, square: u64, piece: Piece, color: Color) -> Result<(), &'static str> {
        if self.is_occupied(square) {
            return Err("that square already has a piece on it");
        }

        match color {
            Color::White => self.white.put(square, piece),
            Color::Black => self.black.put(square, piece),
        }
    }

    pub fn remove(&mut self, square: u64) -> Option<(Piece, Color)> {
        let color = match self.get(square) {
            Some((_piece, color)) => color,
            None => return None,
        };

        let result = match color {
            Color::White => self.white.remove(square),
            Color::Black => self.black.remove(square),
        };

        match result {
            Some(piece) => Some((piece, color)),
            None => None,
        }
    }

    pub fn turn(&self) -> Color {
        self.turn
    }

    pub fn next_turn(&mut self) -> Color {
        self.turn = self.turn.opposite();
        self.turn
    }

    pub fn push_en_passant_target(&mut self, target_square: u64) -> u64 {
        self.en_passant_target_stack.push(target_square);
        target_square
    }

    pub fn peek_en_passant_target(&self) -> u64 {
        *self.en_passant_target_stack.last().unwrap()
    }

    pub fn pop_en_passant_target(&mut self) -> u64 {
        self.en_passant_target_stack.pop().unwrap()
    }

    pub fn preserve_castle_rights(&mut self) -> CastleRightsBitmask {
        let rights = self.peek_castle_rights();
        self.castle_rights_stack.push(rights);
        rights
    }

    pub fn lose_castle_rights(&mut self, lost_rights: CastleRightsBitmask) -> CastleRightsBitmask {
        let old_rights = self.peek_castle_rights();
        let new_rights = old_rights ^ (old_rights & lost_rights);
        self.castle_rights_stack.push(new_rights);
        new_rights
    }

    pub fn peek_castle_rights(&self) -> u8 {
        *self.castle_rights_stack.last().unwrap()
    }

    pub fn pop_castle_rights(&mut self) -> CastleRightsBitmask {
        self.castle_rights_stack.pop().unwrap()
    }

    pub fn material_value(&self) -> u16 {
        self.white.material_value() - self.black.material_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_starting_material_value() {
        let board = Board::starting_position();
        assert_eq!(0, board.material_value());
    }
}
