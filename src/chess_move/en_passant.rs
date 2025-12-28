use core::fmt;

use common::bitboard::Square;

use crate::board::{color::Color, error::BoardError, piece::Piece, Board};

use super::capture::Capture;
use super::chess_move_effect::ChessMoveEffect;
use super::traits::ChessMoveType;

/// Represents an en passant chess move.
#[derive(PartialEq, Clone, Eq, PartialOrd, Ord)]
pub struct EnPassantChessMove {
    /// The square the pawn is moving from.
    from_square: Square,

    /// The square the pawn is moving to.
    to_square: Square,

    effect: Option<ChessMoveEffect>,
}

impl EnPassantChessMove {
    pub fn new(from_square: Square, to_square: Square) -> Self {
        Self {
            from_square,
            to_square,
            effect: None,
        }
    }

    pub fn to_square(&self) -> Square {
        self.to_square
    }

    pub fn from_square(&self) -> Square {
        self.from_square
    }

    pub fn captures(&self) -> Capture {
        Capture(Piece::Pawn)
    }

    pub fn effect(&self) -> Option<ChessMoveEffect> {
        self.effect
    }

    pub fn set_effect(&mut self, effect: ChessMoveEffect) {
        self.effect = Some(effect);
    }

    fn captures_square(&self, color: Color) -> Square {
        let to_bb = self.to_square.to_bitboard();
        let captures_bb = match color {
            Color::White => to_bb >> 8,
            Color::Black => to_bb << 8,
        };
        captures_bb.to_square()
    }

    #[must_use = "move application may fail"]
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
        let captures_square = self.captures_square(color);

        if board.remove(captures_square).is_none() {
            return Err(BoardError::EnPassantDidNotResultInCaptureError {
                chess_move: self.clone(),
            });
        }

        board.reset_halfmove_clock();
        board.increment_fullmove_clock();
        board.push_en_passant_target(None);
        board.preserve_castle_rights();
        board.put(*to_square, piece_to_move, color)?;

        Ok(())
    }

    #[must_use = "move undo may fail"]
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
        let captures_square = self.captures_square(piece_color);

        // Revert the board state.
        board.pop_halfmove_clock();
        board.decrement_fullmove_clock();
        board.pop_en_passant_target();
        board.pop_castle_rights();
        board.put(captures_square, Piece::Pawn, piece_color.opposite())?;

        Ok(())
    }
}

impl ChessMoveType for EnPassantChessMove {
    fn from_square(&self) -> Square {
        self.from_square
    }

    fn to_square(&self) -> Square {
        self.to_square
    }

    fn effect(&self) -> Option<ChessMoveEffect> {
        self.effect
    }

    fn set_effect(&mut self, effect: ChessMoveEffect) {
        self.effect = Some(effect);
    }

    fn apply(&self, board: &mut Board) -> Result<(), BoardError> {
        EnPassantChessMove::apply(self, board)
    }

    fn undo(&self, board: &mut Board) -> Result<(), BoardError> {
        EnPassantChessMove::undo(self, board)
    }
}

impl fmt::Display for EnPassantChessMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let check_or_checkmate_msg = match self.effect() {
            Some(ChessMoveEffect::Check) => "check",
            Some(ChessMoveEffect::Checkmate) => "checkmate",
            _ => "",
        };
        write!(
            f,
            "en passant {} {}{}",
            self.from_square.to_algebraic(),
            self.to_square.to_algebraic(),
            check_or_checkmate_msg,
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
    ($from:expr, $to:expr) => {{
        let mut chess_move = ChessMove::EnPassant(EnPassantChessMove::new($from, $to));
        chess_move.set_effect(ChessMoveEffect::None);
        chess_move
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess_move::chess_move::ChessMove;
    use crate::chess_move::standard::StandardChessMove;
    use crate::{chess_position, std_move};
    use common::bitboard::*;

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
        assert_eq!(Some(D3), board.peek_en_passant_target());

        let en_passant = en_passant_move!(E4, D3);
        en_passant.apply(&mut board).unwrap();
        println!("After en passant:\n{}", board);
        assert_eq!(Some((Piece::Pawn, Color::Black)), board.get(D3));
        assert_eq!(None, board.get(D4));
        assert_eq!(None, board.peek_en_passant_target());

        en_passant.undo(&mut board).unwrap();
        println!("Undo en passant:\n{}", board);
        assert_eq!(Some((Piece::Pawn, Color::White)), board.get(D4));
        assert_eq!(Some((Piece::Pawn, Color::Black)), board.get(E4));
        assert_eq!(Some(D3), board.peek_en_passant_target());
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
