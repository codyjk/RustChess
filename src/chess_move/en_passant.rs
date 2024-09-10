use core::fmt;

use common::bitboard::{bitboard::Bitboard, square};

use crate::board::{color::Color, error::BoardError, piece::Piece, Board};

use super::Capture;

/// Represents an en passant chess move.
#[derive(PartialEq, Clone, Eq, PartialOrd, Ord)]
pub struct EnPassantChessMove {
    /// The square the pawn is moving from.
    from_square: Bitboard,

    /// The square the pawn is moving to.
    to_square: Bitboard,
}

impl EnPassantChessMove {
    pub fn new(from_square: Bitboard, to_square: Bitboard) -> Self {
        Self {
            from_square,
            to_square,
        }
    }

    pub fn to_square(&self) -> Bitboard {
        self.to_square
    }

    pub fn from_square(&self) -> Bitboard {
        self.from_square
    }

    pub fn captures(&self) -> Capture {
        Capture(Piece::Pawn)
    }

    pub fn apply(&self, board: &mut Board) -> Result<(), BoardError> {
        let EnPassantChessMove {
            from_square,
            to_square,
            ..
        } = self;
        let maybe_piece = board.remove(*from_square);
        let (piece_to_move, color) = match maybe_piece {
            None => return Err(BoardError::FromSquareIsEmptyMoveApplicationError),
            Some((piece, color)) => (piece, color),
        };

        if piece_to_move != Piece::Pawn {
            return Err(BoardError::EnPassantNonPawnMoveApplicationError);
        }

        // the captured pawn is "behind" the target square
        let captures_square = match color {
            Color::White => *to_square >> 8,
            Color::Black => *to_square << 8,
        };

        if board.remove(captures_square).is_none() {
            return Err(BoardError::EnPassantDidNotResultInCaptureError {
                chess_move: self.clone(),
            });
        }

        board.reset_halfmove_clock();
        board.increment_fullmove_clock();
        board.push_en_passant_target(Bitboard::EMPTY);
        board.preserve_castle_rights();
        board.put(*to_square, piece_to_move, color)?;

        Ok(())
    }

    pub fn undo(&self, board: &mut Board) -> Result<(), BoardError> {
        let EnPassantChessMove {
            from_square,
            to_square,
            ..
        } = self;

        // remove the moved pawn
        let maybe_piece = board.remove(*to_square);
        let (piece_to_move_back, piece_color) = match maybe_piece {
            None => return Err(BoardError::ToSquareIsEmptyMoveUndoError),
            Some((piece, color)) => (piece, color),
        };

        if piece_to_move_back != Piece::Pawn {
            return Err(BoardError::EnPassantNonPawnMoveUndoError);
        }

        // return the pawn to its original square
        board
            .put(*from_square, piece_to_move_back, piece_color)
            .unwrap();

        // the captured pawn is "behind" the target square
        let captures_square = match piece_color {
            Color::White => *to_square >> 8,
            Color::Black => *to_square << 8,
        };

        // Revert the board state.
        board.pop_halfmove_clock();
        board.decrement_fullmove_clock();
        board.pop_en_passant_target();
        board.pop_castle_rights();
        board.put(captures_square, Piece::Pawn, piece_color.opposite())?;

        Ok(())
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
macro_rules! en_passant_move {
    ($from:expr, $to:expr) => {
        ChessMove::EnPassant(EnPassantChessMove::new($from, $to))
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess_move::standard::StandardChessMove;
    use crate::chess_move::ChessMove;
    use crate::{chess_position, std_move};
    use common::bitboard::square::*;

    #[test]
    fn test_apply_and_undo_en_passant() {
        let mut board = chess_position! {
            ........
            ........
            ........
            ........
            ....p...
            ........
            ...P....
            ........
        };
        println!("Testing board:\n{}", board);

        let standard_move_revealing_ep = std_move!(D2, D4);
        standard_move_revealing_ep.apply(&mut board).unwrap();
        println!("After move that reveals en passant:\n{}", board);
        assert_eq!(Some((Piece::Pawn, Color::White)), board.get(D4));
        assert_eq!(D3, board.peek_en_passant_target());

        let en_passant = en_passant_move!(E4, D3);
        en_passant.apply(&mut board).unwrap();
        println!("After en passant:\n{}", board);
        assert_eq!(Some((Piece::Pawn, Color::Black)), board.get(D3));
        assert_eq!(None, board.get(D4));
        assert_eq!(Bitboard::EMPTY, board.peek_en_passant_target());

        en_passant.undo(&mut board).unwrap();
        println!("Undo en passant:\n{}", board);
        assert_eq!(Some((Piece::Pawn, Color::White)), board.get(D4));
        assert_eq!(Some((Piece::Pawn, Color::Black)), board.get(E4));
        assert_eq!(D3, board.peek_en_passant_target());
    }

    #[test]
    fn test_zobrist_hashing_reversible_for_en_passant() {
        let mut board = chess_position! {
            ........
            ........
            ........
            ........
            ....p...
            ........
            ...P....
            ........
        };
        let initial_hash = board.current_position_hash();

        let standard_move_revealing_ep = std_move!(D2, D4);
        standard_move_revealing_ep.apply(&mut board).unwrap();
        assert_ne!(initial_hash, board.current_position_hash());

        let en_passant = en_passant_move!(E4, D3);
        en_passant.apply(&mut board).unwrap();
        assert_ne!(initial_hash, board.current_position_hash());

        en_passant.undo(&mut board).unwrap();
        standard_move_revealing_ep.undo(&mut board).unwrap();
        assert_eq!(initial_hash, board.current_position_hash());
    }
}
