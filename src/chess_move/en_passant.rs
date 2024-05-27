use core::fmt;

use crate::board::{bitboard::EMPTY, color::Color, error::BoardError, piece::Piece, square, Board};

use super::{Capture, ChessMove};

#[derive(PartialEq, Clone)]
pub struct EnPassantChessMove {
    from_square: u64,
    to_square: u64,
}

impl EnPassantChessMove {
    pub fn new(from_square: u64, to_square: u64) -> Self {
        Self {
            from_square,
            to_square,
        }
    }
}

impl ChessMove for EnPassantChessMove {
    fn to_square(&self) -> u64 {
        self.to_square
    }

    fn from_square(&self) -> u64 {
        self.from_square
    }

    fn capture(&self) -> Option<Capture> {
        None
    }

    fn apply(&self, board: &mut Board) -> Result<Option<Capture>, BoardError> {
        let EnPassantChessMove {
            from_square,
            to_square,
        } = self;
        let maybe_piece = board.remove(*from_square);
        let (piece_to_move, color) = match maybe_piece {
            None => return Err(BoardError::FromSquareIsEmpty { op: "apply" }),
            Some((piece, color)) => (piece, color),
        };

        if piece_to_move != Piece::Pawn {
            return Err(BoardError::EnPassantNonPawn { op: "apply" });
        }

        // the captured pawn is "behind" the target square
        let capture_square = match color {
            Color::White => to_square >> 8,
            Color::Black => to_square << 8,
        };

        let capture = match board.remove(capture_square) {
            Some((piece, color)) => (piece, color),
            None => return Err(BoardError::EnPassantNonCapture),
        };

        board.reset_halfmove_clock();
        board.increment_fullmove_clock();
        board.push_en_passant_target(EMPTY);
        board.preserve_castle_rights();
        board
            .put(*to_square, piece_to_move, color)
            .map(|_| Some(capture))
    }

    fn undo(&self, board: &mut Board) -> Result<Option<Capture>, BoardError> {
        let EnPassantChessMove {
            from_square,
            to_square,
        } = self;

        // remove the moved pawn
        let maybe_piece = board.remove(*to_square);
        let (piece_to_move_back, piece_color) = match maybe_piece {
            None => return Err(BoardError::ToSquareIsEmpty { op: "undo" }),
            Some((piece, color)) => (piece, color),
        };

        if piece_to_move_back != Piece::Pawn {
            return Err(BoardError::EnPassantNonPawn { op: "undo" });
        }

        // return the pawn to its original square
        board
            .put(*from_square, piece_to_move_back, piece_color)
            .unwrap();

        // the captured pawn is "behind" the target square
        let capture_square = match piece_color {
            Color::White => to_square >> 8,
            Color::Black => to_square << 8,
        };

        // Revert the board state.
        board.pop_halfmove_clock();
        board.decrement_fullmove_clock();
        board.pop_en_passant_target();
        board.pop_castle_rights();
        board
            .put(capture_square, Piece::Pawn, piece_color.opposite())
            .map(|_| None)
    }
}

impl fmt::Display for EnPassantChessMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "en passant {} {}",
            square::to_algebraic(self.from_square).to_lowercase(),
            square::to_algebraic(self.to_square).to_lowercase(),
        )
    }
}

impl fmt::Debug for EnPassantChessMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format!("{}", self).fmt(f)
    }
}

#[macro_export]
macro_rules! en_passant {
    ($from:expr, $to:expr) => {
        EnPassantChessMove::new($from, $to)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess_move::standard::StandardChessMove;
    use crate::{
        board::square::{D2, D3, D4, E4},
        std_move,
    };

    #[test]
    fn test_apply_and_undo_en_passant() {
        let mut board = Board::new();
        board.put(D2, Piece::Pawn, Color::White).unwrap();
        board.put(E4, Piece::Pawn, Color::Black).unwrap();
        println!("Testing board:\n{}", board);

        let standard_move_revealing_ep = std_move!(D2, D4);
        standard_move_revealing_ep.apply(&mut board).unwrap();
        println!("After move that reveals en passant:\n{}", board);
        assert_eq!(Some((Piece::Pawn, Color::White)), board.get(D4));
        assert_eq!(D3, board.peek_en_passant_target());

        let en_passant = en_passant!(E4, D3);
        en_passant.apply(&mut board).unwrap();
        println!("After en passant:\n{}", board);
        assert_eq!(Some((Piece::Pawn, Color::Black)), board.get(D3));
        assert_eq!(None, board.get(D4));
        assert_eq!(EMPTY, board.peek_en_passant_target());

        en_passant.undo(&mut board).unwrap();
        println!("Undo en passant:\n{}", board);
        assert_eq!(Some((Piece::Pawn, Color::White)), board.get(D4));
        assert_eq!(Some((Piece::Pawn, Color::Black)), board.get(E4));
        assert_eq!(D3, board.peek_en_passant_target());
    }

    #[test]
    fn test_zobrist_hashing_reversible_for_en_passant() {
        let mut board = Board::new();
        board.put(D2, Piece::Pawn, Color::White).unwrap();
        board.put(E4, Piece::Pawn, Color::Black).unwrap();
        let initial_hash = board.current_position_hash();

        let standard_move_revealing_ep = std_move!(D2, D4);
        standard_move_revealing_ep.apply(&mut board).unwrap();
        assert_ne!(initial_hash, board.current_position_hash());

        let en_passant = en_passant!(E4, D3);
        en_passant.apply(&mut board).unwrap();
        assert_ne!(initial_hash, board.current_position_hash());

        en_passant.undo(&mut board).unwrap();
        standard_move_revealing_ep.undo(&mut board).unwrap();
        assert_eq!(initial_hash, board.current_position_hash());
    }
}
