use core::fmt;

use crate::board::{error::BoardError, piece::Piece, square, Board};

use super::{standard::StandardChessMove, Capture, ChessMove};

#[derive(PartialEq, Clone)]
pub struct PawnPromotionChessMove {
    from_square: u64,
    to_square: u64,
    capture: Option<Capture>,
    promote_to_piece: Piece,
}

impl PawnPromotionChessMove {
    pub fn new(
        from_square: u64,
        to_square: u64,
        capture: Option<Capture>,
        promote_to_piece: Piece,
    ) -> Self {
        Self {
            from_square,
            to_square,
            capture,
            promote_to_piece,
        }
    }
}

impl ChessMove for PawnPromotionChessMove {
    fn to_square(&self) -> u64 {
        self.to_square
    }

    fn from_square(&self) -> u64 {
        self.from_square
    }

    fn capture(&self) -> Option<Capture> {
        self.capture
    }

    fn apply(&self, board: &mut Board) -> Result<Option<Capture>, BoardError> {
        let PawnPromotionChessMove {
            from_square,
            to_square,
            capture: expected_capture,
            promote_to_piece,
        } = self;

        // This is a special case. It's like a standard move, but we replace the
        // pawn at the end. So apply the standard move first.
        let standard_move = StandardChessMove::new(*from_square, *to_square, *expected_capture);
        let capture = standard_move.apply(board)?;

        // Then, we perform the promotion.
        match board.remove(*to_square) {
            Some((Piece::Pawn, color)) => {
                board.put(*to_square, *promote_to_piece, color)?;
            }
            _ => return Err(BoardError::PromotionNonPawnError),
        }

        // Return the capture result from the standard move.
        Ok(capture)
    }

    fn undo(&self, board: &mut Board) -> Result<Option<Capture>, BoardError> {
        let PawnPromotionChessMove {
            from_square,
            to_square,
            capture: expected_capture,
            promote_to_piece,
        } = self;

        // Undo the promotion first.
        match board.remove(*to_square) {
            Some((piece, color)) if piece == *promote_to_piece => {
                board.put(*to_square, Piece::Pawn, color)?;
            }
            _ => return Err(BoardError::PromotionNonPawnError),
        }

        // Then, undo the standard move.
        let standard_move = StandardChessMove::new(*from_square, *to_square, *expected_capture);
        standard_move.undo(board)
    }
}

impl fmt::Display for PawnPromotionChessMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let capture_msg = match self.capture {
            Some((piece, color)) => format!(" (captures {})", piece.to_fen(color)),
            None => "".to_string(),
        };

        write!(
            f,
            "promote {}{}{}",
            square::to_algebraic(self.from_square).to_lowercase(),
            square::to_algebraic(self.to_square).to_lowercase(),
            capture_msg
        )
    }
}

impl fmt::Debug for PawnPromotionChessMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format!("{}", self).fmt(f)
    }
}

#[macro_export]
macro_rules! promotion {
    ($from:expr, $to:expr, $capture:expr, $piece:expr) => {
        PawnPromotionChessMove::new($from, $to, $capture, $piece)
    };
}

#[cfg(test)]
mod tests {
    use crate::board::{
        color::Color,
        square::{A7, A8, B8},
    };

    use super::*;

    #[test]
    fn test_apply_and_undo_pawn_promotion() {
        let mut board = Board::new();
        board.put(A7, Piece::Pawn, Color::White).unwrap();
        println!("Testing board:\n{}", board);

        let promotion = PawnPromotionChessMove::new(A7, A8, None, Piece::Queen);

        promotion.apply(&mut board).unwrap();
        println!("After applying promotion:\n{}", board);
        assert_eq!(None, board.get(A7));
        assert_eq!(Some((Piece::Queen, Color::White)), board.get(A8));

        promotion.undo(&mut board).unwrap();
        println!("After undoing promotion:\n{}", board);
        assert_eq!(Some((Piece::Pawn, Color::White)), board.get(A7));
        assert_eq!(None, board.get(A8));
    }

    #[test]
    fn test_apply_and_undo_pawn_promotion_with_capture() {
        let mut board = Board::new();
        board.put(A7, Piece::Pawn, Color::White).unwrap();
        board.put(B8, Piece::Rook, Color::Black).unwrap();
        println!("Testing board:\n{}", board);

        let promotion =
            PawnPromotionChessMove::new(A7, B8, Some((Piece::Rook, Color::Black)), Piece::Queen);

        promotion.apply(&mut board).unwrap();
        println!("After applying promotion:\n{}", board);
        assert_eq!(None, board.get(A7));
        assert_eq!(Some((Piece::Queen, Color::White)), board.get(B8));

        promotion.undo(&mut board).unwrap();
        println!("After undoing promotion:\n{}", board);
        assert_eq!(Some((Piece::Pawn, Color::White)), board.get(A7));
        assert_eq!(Some((Piece::Rook, Color::Black)), board.get(B8));
    }

    #[test]
    fn test_zobrist_hashing_reversible_for_pawn_promotion() {
        let mut board = Board::new();
        board.put(A7, Piece::Pawn, Color::White).unwrap();
        let initial_hash = board.current_position_hash();

        let promotion = PawnPromotionChessMove::new(A7, A8, None, Piece::Queen);
        promotion.apply(&mut board).unwrap();
        assert_ne!(initial_hash, board.current_position_hash());

        promotion.undo(&mut board).unwrap();
        assert_eq!(initial_hash, board.current_position_hash());
    }
}
