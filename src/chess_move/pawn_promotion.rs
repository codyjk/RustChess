use core::fmt;

use common::bitboard::{bitboard::Bitboard, square};

use crate::board::{error::BoardError, piece::Piece, Board};

use super::capture::Capture;
use super::chess_move_effect::ChessMoveEffect;
use super::standard::StandardChessMove;

/// Represents a pawn promotion chess move. The board logic is implemented as
/// a superset of a standard pawn move, but at the end, the pawn is replaced
/// with the promotion piece.
#[derive(PartialEq, Clone, Eq, PartialOrd, Ord)]
pub struct PawnPromotionChessMove {
    from_square: Bitboard,
    to_square: Bitboard,
    captures: Option<Capture>,
    promote_to_piece: Piece,
    effect: Option<ChessMoveEffect>,
}

impl PawnPromotionChessMove {
    pub fn new(
        from_square: Bitboard,
        to_square: Bitboard,
        captures: Option<Capture>,
        promote_to_piece: Piece,
    ) -> Self {
        Self {
            from_square,
            to_square,
            captures,
            promote_to_piece,
            effect: None,
        }
    }

    pub fn to_square(&self) -> Bitboard {
        self.to_square
    }

    pub fn from_square(&self) -> Bitboard {
        self.from_square
    }

    pub fn captures(&self) -> Option<Capture> {
        self.captures
    }

    pub fn effect(&self) -> Option<ChessMoveEffect> {
        self.effect
    }

    pub fn set_effect(&mut self, effect: ChessMoveEffect) {
        self.effect = Some(effect);
    }

    pub fn promote_to_piece(&self) -> Piece {
        self.promote_to_piece
    }

    pub fn apply(&self, board: &mut Board) -> Result<(), BoardError> {
        let PawnPromotionChessMove {
            from_square,
            to_square,
            captures: expected_captures,
            promote_to_piece,
            ..
        } = self;

        // This is a special case. It's like a standard move, but we replace the
        // pawn at the end. So apply the standard move first.
        let standard_move = StandardChessMove::new(*from_square, *to_square, *expected_captures);
        standard_move.apply(board)?;

        // Then, we perform the promotion.
        match board.remove(*to_square) {
            Some((Piece::Pawn, color)) => {
                board.put(*to_square, *promote_to_piece, color)?;
            }
            _ => return Err(BoardError::PromotionNonPawnError),
        }

        Ok(())
    }

    pub fn undo(&self, board: &mut Board) -> Result<(), BoardError> {
        let PawnPromotionChessMove {
            from_square,
            to_square,
            captures: expected_captures,
            promote_to_piece,
            ..
        } = self;

        // Undo the promotion first.
        match board.remove(*to_square) {
            Some((piece, color)) if piece == *promote_to_piece => {
                board.put(*to_square, Piece::Pawn, color)?;
            }
            _ => return Err(BoardError::PromotionNonPawnError),
        }

        // Then, undo the standard move.
        let standard_move = StandardChessMove::new(*from_square, *to_square, *expected_captures);
        standard_move.undo(board)?;

        Ok(())
    }
}

impl fmt::Display for PawnPromotionChessMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let captures_msg = match self.captures {
            Some(capture) => format!(" (captures {})", capture.0),
            None => "".to_string(),
        };
        let check_or_checkmate_msg = match self.effect() {
            Some(ChessMoveEffect::Check) => "check",
            Some(ChessMoveEffect::Checkmate) => "checkmate",
            _ => "",
        };

        write!(
            f,
            "promote {}{}{}{}",
            square::to_algebraic(self.from_square).to_lowercase(),
            square::to_algebraic(self.to_square).to_lowercase(),
            captures_msg,
            check_or_checkmate_msg,
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
    ($from:expr, $to:expr, $captures:expr, $piece:expr) => {{
        let mut chess_move =
            ChessMove::PawnPromotion(PawnPromotionChessMove::new($from, $to, $captures, $piece));
        chess_move.set_effect(ChessMoveEffect::None);
        chess_move
    }};
}

#[cfg(test)]
mod tests {
    use crate::{board::color::Color, chess_position};
    use common::bitboard::square::*;

    use super::*;

    #[test]
    fn test_apply_and_undo_pawn_promotion() {
        let mut board = chess_position! {
            ........
            P.......
            ........
            ........
            ........
            ........
            ........
            ........
        };
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
        let mut board = chess_position! {
            .r......
            P.......
            ........
            ........
            ........
            ........
            ........
            ........
        };
        println!("Testing board:\n{}", board);

        let promotion =
            PawnPromotionChessMove::new(A7, B8, Some(Capture(Piece::Rook)), Piece::Queen);

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
        let mut board = chess_position! {
            ........
            P.......
            ........
            ........
            ........
            ........
            ........
            ........
        };
        let initial_hash = board.current_position_hash();

        let promotion = PawnPromotionChessMove::new(A7, A8, None, Piece::Queen);
        promotion.apply(&mut board).unwrap();
        assert_ne!(initial_hash, board.current_position_hash());

        promotion.undo(&mut board).unwrap();
        assert_eq!(initial_hash, board.current_position_hash());
    }
}
