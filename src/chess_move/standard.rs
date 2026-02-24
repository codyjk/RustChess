use core::fmt;

use crate::board::{
    castle_rights::CastleRights, color::Color, error::BoardError, piece::Piece, Board,
};
use common::bitboard::{
    bitboard::Bitboard,
    square::{self, *},
};
use log::debug;
#[cfg(feature = "instrumentation")]
use tracing::instrument;

use super::{
    capture::Capture, chess_move_effect::ChessMoveEffect, pawn_promotion::PawnPromotionChessMove,
    traits::ChessMoveType,
};

/// Represents a standard chess move. A standard move is a move that does not involve
/// pawn promotion, en passant, or castling.
#[derive(Clone, Eq, PartialOrd, Ord)]
pub struct StandardChessMove {
    from_square: square::Square,
    to_square: square::Square,
    captures: Option<Capture>,
    effect: Option<ChessMoveEffect>,
}

impl PartialEq for StandardChessMove {
    fn eq(&self, other: &Self) -> bool {
        self.from_square == other.from_square
            && self.to_square == other.to_square
            && self.captures == other.captures
    }
}

impl StandardChessMove {
    pub fn new(
        from_square: square::Square,
        to_square: square::Square,
        captures: Option<Capture>,
    ) -> Self {
        Self {
            from_square,
            to_square,
            captures,
            effect: None,
        }
    }

    pub fn to_square(&self) -> square::Square {
        self.to_square
    }

    pub fn from_square(&self) -> square::Square {
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

    #[must_use = "move application may fail"]
    #[cfg_attr(feature = "instrumentation", instrument(skip_all))]
    pub fn apply(&self, board: &mut Board) -> Result<(), BoardError> {
        let StandardChessMove {
            from_square,
            to_square,
            captures,
            ..
        } = self;

        let (piece_to_move, color_of_piece_to_move) = board
            .remove(*from_square)
            .ok_or(BoardError::FromSquareIsEmptyMoveApplicationError)?;

        let captured_piece_and_color = board.remove(*to_square);
        let expected_capture_piece_and_color =
            captures.map(|capture| (capture.0, color_of_piece_to_move.opposite()));
        if captured_piece_and_color != expected_capture_piece_and_color {
            debug!("captured piece and color: {:?}", captured_piece_and_color);
            debug!(
                "expected capture piece and color: {:?}",
                expected_capture_piece_and_color
            );
            return Err(BoardError::UnexpectedCaptureResultError);
        }

        let en_passant_target = get_en_passant_target_square(
            piece_to_move,
            color_of_piece_to_move,
            *from_square,
            *to_square,
        );
        let lost_castle_rights =
            get_lost_castle_rights_if_rook_or_king_moved(
                piece_to_move,
                color_of_piece_to_move,
                *from_square,
            ) | get_lost_castle_rights_if_rook_taken(captured_piece_and_color, *to_square);

        if captured_piece_and_color.is_some() {
            board.reset_halfmove_clock();
        } else {
            board.increment_halfmove_clock();
        }

        board.increment_fullmove_clock();
        board.push_en_passant_target(en_passant_target);
        board.lose_castle_rights(lost_castle_rights);
        board
            .put(*to_square, piece_to_move, color_of_piece_to_move)
            .expect("to_square should be empty after removing piece");

        Ok(())
    }

    #[must_use = "move undo may fail"]
    #[cfg_attr(feature = "instrumentation", instrument(skip_all))]
    pub fn undo(&self, board: &mut Board) -> Result<(), BoardError> {
        let StandardChessMove {
            from_square,
            to_square,
            captures,
            ..
        } = self;

        // Remove the moved piece.
        let (piece_to_move_back, color_of_piece_to_move_back) = board
            .remove(*to_square)
            .ok_or(BoardError::ToSquareIsEmptyMoveUndoError)?;

        // Put the captured piece back.
        if let Some(captures) = captures {
            board.put(
                *to_square,
                captures.0,
                color_of_piece_to_move_back.opposite(),
            )?;
        }

        // Revert the board state.
        board.pop_halfmove_clock();
        board.decrement_fullmove_clock();
        board.pop_en_passant_target();
        board.pop_castle_rights();
        board
            .put(
                *from_square,
                piece_to_move_back,
                color_of_piece_to_move_back,
            )
            .expect("from_square should be empty when undoing move");

        Ok(())
    }

    // In move generation, we convert standard pawn moves on the final rank
    // to pawn promotion moves. These are helpers used in the move generation.

    pub fn is_promotable_pawn(&self, board: &Board) -> bool {
        let pawn_color = match board.get(self.from_square) {
            Some((Piece::Pawn, color)) => color,
            _ => return false,
        };
        let (overlaps_back_rank, overlaps_promotion_rank) = match pawn_color {
            Color::White => (
                self.from_square.overlaps(Bitboard::RANK_7),
                self.to_square.overlaps(Bitboard::RANK_8),
            ),
            Color::Black => (
                self.from_square.overlaps(Bitboard::RANK_2),
                self.to_square.overlaps(Bitboard::RANK_1),
            ),
        };

        overlaps_back_rank && overlaps_promotion_rank
    }

    pub fn to_pawn_promotion(
        &self,
        board: &Board,
        promote_to_piece: Piece,
    ) -> Result<PawnPromotionChessMove, BoardError> {
        if !self.is_promotable_pawn(board) {
            return Err(BoardError::PawnNotPromotableError);
        }

        Ok(PawnPromotionChessMove::new(
            self.from_square,
            self.to_square,
            self.captures,
            promote_to_piece,
        ))
    }
}

impl ChessMoveType for StandardChessMove {
    fn from_square(&self) -> square::Square {
        self.from_square
    }

    fn to_square(&self) -> square::Square {
        self.to_square
    }

    fn effect(&self) -> Option<ChessMoveEffect> {
        self.effect
    }

    fn set_effect(&mut self, effect: ChessMoveEffect) {
        self.effect = Some(effect);
    }

    fn apply(&self, board: &mut Board) -> Result<(), BoardError> {
        StandardChessMove::apply(self, board)
    }

    fn undo(&self, board: &mut Board) -> Result<(), BoardError> {
        StandardChessMove::undo(self, board)
    }
}

/// Determines if a move creates an en passant opportunity. If so, returns the target square.
fn get_en_passant_target_square(
    piece_to_move: Piece,
    color: Color,
    from_square: square::Square,
    to_square: square::Square,
) -> Option<square::Square> {
    if piece_to_move != Piece::Pawn {
        return None;
    }

    let is_double_push = match color {
        Color::White => {
            from_square.overlaps(Bitboard::RANK_2) && to_square.overlaps(Bitboard::RANK_4)
        }
        Color::Black => {
            from_square.overlaps(Bitboard::RANK_7) && to_square.overlaps(Bitboard::RANK_5)
        }
    };

    if !is_double_push {
        return None;
    }

    // The en passant target is the square the pawn passed over
    let target_index = match color {
        Color::White => from_square.index() + 8,
        Color::Black => from_square.index() - 8,
    };
    Some(square::Square::new(target_index))
}

fn get_lost_castle_rights_if_rook_or_king_moved(
    piece_to_move: Piece,
    color: Color,
    from_square: square::Square,
) -> CastleRights {
    match (piece_to_move, color, from_square) {
        (Piece::Rook, Color::White, sq) if sq == A1 => CastleRights::white_queenside(),
        (Piece::Rook, Color::White, sq) if sq == H1 => CastleRights::white_kingside(),
        (Piece::Rook, Color::Black, sq) if sq == A8 => CastleRights::black_queenside(),
        (Piece::Rook, Color::Black, sq) if sq == H8 => CastleRights::black_kingside(),
        (Piece::King, Color::White, sq) if sq == E1 => {
            CastleRights::white_kingside() | CastleRights::white_queenside()
        }
        (Piece::King, Color::Black, sq) if sq == E8 => {
            CastleRights::black_kingside() | CastleRights::black_queenside()
        }
        _ => CastleRights::none(),
    }
}

fn get_lost_castle_rights_if_rook_taken(
    captured_piece: Option<(Piece, Color)>,
    to_square: square::Square,
) -> CastleRights {
    match (captured_piece, to_square) {
        (Some((Piece::Rook, Color::White)), sq) if sq == A1 => CastleRights::white_queenside(),
        (Some((Piece::Rook, Color::White)), sq) if sq == H1 => CastleRights::white_kingside(),
        (Some((Piece::Rook, Color::Black)), sq) if sq == A8 => CastleRights::black_queenside(),
        (Some((Piece::Rook, Color::Black)), sq) if sq == H8 => CastleRights::black_kingside(),
        _ => CastleRights::none(),
    }
}

impl fmt::Display for StandardChessMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let captures_msg = match self.captures {
            Some(capture) => format!(" (captures {})", capture.0),
            None => "".to_string(),
        };
        let check_or_checkmate_msg = match self.effect {
            Some(ChessMoveEffect::Check) => " (check)",
            Some(ChessMoveEffect::Checkmate) => " (checkmate)",
            _ => "",
        };

        write!(
            f,
            "move {}{}{}{}",
            self.from_square.to_algebraic(),
            self.to_square.to_algebraic(),
            captures_msg,
            check_or_checkmate_msg
        )
    }
}

impl fmt::Debug for StandardChessMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format!("{}", self).fmt(f)
    }
}

#[macro_export]
macro_rules! std_move {
    ($from:expr, $to:expr, $captures:expr) => {{
        let mut chess_move =
            ChessMove::Standard(StandardChessMove::new($from, $to, Some($captures)));
        chess_move.set_effect($crate::chess_move::chess_move_effect::ChessMoveEffect::None);
        chess_move
    }};
    ($from:expr, $to:expr) => {{
        let mut chess_move = ChessMove::Standard(StandardChessMove::new($from, $to, None));
        chess_move.set_effect($crate::chess_move::chess_move_effect::ChessMoveEffect::None);
        chess_move
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess_move::chess_move::ChessMove;
    use crate::chess_position;

    #[test]
    fn test_apply_chess_move() {
        let mut board = Board::default();

        // using a queens gambit accepted opening to test basic chess move application
        let moves = vec![
            (E2, E4, (Piece::Pawn, Color::White), None),
            (E7, E5, (Piece::Pawn, Color::Black), None),
            (D2, D4, (Piece::Pawn, Color::White), None),
            (
                E5,
                D4,
                (Piece::Pawn, Color::Black),
                Some(Capture(Piece::Pawn)),
            ),
        ];

        for (from_square, to_square, moved, expected_capture) in &moves {
            let chess_move = StandardChessMove::new(*from_square, *to_square, *expected_capture);
            chess_move.apply(&mut board).unwrap();
            assert_eq!(board.get(*to_square).unwrap(), *moved);
        }
    }

    #[test]
    fn test_undo_pawn_move() {
        let mut board = Board::default();
        let original_board = format!("{}", board);

        let chess_move = std_move!(A2, A4);
        chess_move.apply(&mut board).unwrap();
        chess_move.undo(&mut board).unwrap();

        let result_board = format!("{}", board);
        assert_eq!(original_board, result_board);
    }

    #[test]
    fn test_undo_knight_move() {
        let mut board = Board::default();
        let original_board = format!("{}", board);

        let chess_move = std_move!(B1, C3);
        chess_move.apply(&mut board).unwrap();
        chess_move.undo(&mut board).unwrap();

        let chess_move_2 = std_move!(G1, F3);
        chess_move_2.apply(&mut board).unwrap();
        chess_move_2.undo(&mut board).unwrap();

        let result_board = format!("{}", board);
        assert_eq!(original_board, result_board);
    }

    #[test]
    fn test_undo_capture() {
        let mut board = chess_position! {
            ........
            ........
            ........
            ........
            .p......
            ........
            N.......
            ........
        };
        let capture = std_move!(A2, B4, Capture(Piece::Pawn));

        capture.apply(&mut board).unwrap();
        capture.undo(&mut board).unwrap();

        assert_eq!(Some((Piece::Knight, Color::White)), board.get(A2));
        assert_eq!(Some((Piece::Pawn, Color::Black)), board.get(B4));
    }

    #[test]
    fn test_white_lose_kingside_castle_rights() {
        let mut board = chess_position! {
            ........
            ........
            ........
            ........
            ........
            ........
            ........
            ....K..R
        };

        assert!(!(board.peek_castle_rights() & CastleRights::white_kingside()).is_empty());
        let chess_move = std_move!(H1, H2);
        chess_move.apply(&mut board).unwrap();
        assert_eq!(
            CastleRights::none(),
            board.peek_castle_rights() & CastleRights::white_kingside()
        );
    }

    #[test]
    fn test_white_lose_queenside_castle_rights() {
        let mut board = chess_position! {
            ........
            ........
            ........
            ........
            ........
            ........
            ........
            R...K...
        };

        assert!(!(board.peek_castle_rights() & CastleRights::white_queenside()).is_empty());
        let chess_move = std_move!(A1, A2);
        chess_move.apply(&mut board).unwrap();
        assert_eq!(
            CastleRights::none(),
            board.peek_castle_rights() & CastleRights::white_queenside()
        );
    }

    #[test]
    fn test_black_lose_kingside_castle_rights() {
        let mut board = chess_position! {
            ....k..r
            ........
            ........
            ........
            ........
            ........
            ........
            ........
        };

        assert!(!(board.peek_castle_rights() & CastleRights::black_kingside()).is_empty());
        let chess_move = std_move!(H8, H2);
        chess_move.apply(&mut board).unwrap();
        assert_eq!(
            CastleRights::none(),
            board.peek_castle_rights() & CastleRights::black_kingside()
        );
    }

    #[test]
    fn test_black_lose_queenside_castle_rights() {
        let mut board = chess_position! {
            r...k...
            ........
            ........
            ........
            ........
            ........
            ........
            ........
        };

        assert!(!(board.peek_castle_rights() & CastleRights::black_queenside()).is_empty());
        let chess_move = std_move!(A8, A2);
        chess_move.apply(&mut board).unwrap();
        assert_eq!(
            CastleRights::none(),
            board.peek_castle_rights() & CastleRights::black_queenside()
        );
    }

    #[test]
    fn test_white_lose_queenside_castle_rights_from_capture() {
        let mut board = chess_position! {
            .......b
            ........
            ........
            ........
            ........
            ........
            ........
            R...K..R
        };

        assert!(!(board.peek_castle_rights() & CastleRights::white_queenside()).is_empty());
        let chess_move = std_move!(H8, A1, Capture(Piece::Rook));
        chess_move.apply(&mut board).unwrap();
        assert_eq!(
            CastleRights::none(),
            board.peek_castle_rights() & CastleRights::white_queenside()
        );
    }

    #[test]
    fn test_white_lose_kingside_castle_rights_from_capture() {
        let mut board = chess_position! {
            b.......
            ........
            ........
            ........
            ........
            ........
            ........
            R...K..R
        };

        assert!(!(board.peek_castle_rights() & CastleRights::white_kingside()).is_empty());
        let chess_move = std_move!(A8, H1, Capture(Piece::Rook));
        chess_move.apply(&mut board).unwrap();
        assert_eq!(
            CastleRights::none(),
            board.peek_castle_rights() & CastleRights::white_kingside()
        );
    }

    #[test]
    fn test_black_lose_queenside_castle_rights_from_capture() {
        let mut board = chess_position! {
            r...k..r
            ........
            ........
            ........
            ........
            ........
            ........
            .......B
        };

        assert!(!(board.peek_castle_rights() & CastleRights::black_queenside()).is_empty());
        let chess_move = std_move!(H1, A8, Capture(Piece::Rook));
        chess_move.apply(&mut board).unwrap();
        assert_eq!(
            CastleRights::none(),
            board.peek_castle_rights() & CastleRights::black_queenside()
        );
    }

    #[test]
    fn test_black_lose_kingside_castle_rights_from_capture() {
        let mut board = chess_position! {
            r...k..r
            ........
            ........
            ........
            ........
            ........
            ........
            B.......
        };

        assert!(!(board.peek_castle_rights() & CastleRights::black_kingside()).is_empty());
        let chess_move = std_move!(A1, H8, Capture(Piece::Rook));
        chess_move.apply(&mut board).unwrap();
        assert_eq!(
            CastleRights::none(),
            board.peek_castle_rights() & CastleRights::black_kingside()
        );
    }

    #[test]
    fn test_white_lose_all_castle_rights() {
        let mut board = chess_position! {
            ........
            ........
            ........
            ........
            ........
            ........
            ........
            R...K..R
        };

        assert!(!(board.peek_castle_rights() & CastleRights::white_kingside()).is_empty());
        assert!(!(board.peek_castle_rights() & CastleRights::white_queenside()).is_empty());
        let chess_move = std_move!(E1, E2);
        chess_move.apply(&mut board).unwrap();
        assert_eq!(
            CastleRights::none(),
            board.peek_castle_rights() & CastleRights::white_kingside()
        );
        assert_eq!(
            CastleRights::none(),
            board.peek_castle_rights() & CastleRights::white_queenside()
        );
    }

    #[test]
    fn test_black_lose_all_castle_rights() {
        let mut board = chess_position! {
            r...k..r
            ........
            ........
            ........
            ........
            ........
            ........
            ........
        };

        assert!(!(board.peek_castle_rights() & CastleRights::black_kingside()).is_empty());
        assert!(!(board.peek_castle_rights() & CastleRights::black_queenside()).is_empty());
        let chess_move = std_move!(E8, E7);
        chess_move.apply(&mut board).unwrap();
        assert_eq!(
            CastleRights::none(),
            board.peek_castle_rights() & CastleRights::black_kingside()
        );
        assert_eq!(
            CastleRights::none(),
            board.peek_castle_rights() & CastleRights::black_queenside()
        );
    }

    #[test]
    fn test_zobrist_hashing_reversible_for_standard_move() {
        let mut board = chess_position! {
            ........
            ........
            ........
            ........
            ........
            ........
            ....P...
            ........
        };
        let initial_hash = board.current_position_hash();

        let chess_move = std_move!(E2, E4);
        chess_move.apply(&mut board).unwrap();
        assert_ne!(initial_hash, board.current_position_hash());

        chess_move.undo(&mut board).unwrap();
        assert_eq!(initial_hash, board.current_position_hash());
    }
}
