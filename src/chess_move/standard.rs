use core::fmt;

use crate::board::{
    castle_rights::{
        BLACK_KINGSIDE_RIGHTS, BLACK_QUEENSIDE_RIGHTS, WHITE_KINGSIDE_RIGHTS,
        WHITE_QUEENSIDE_RIGHTS,
    },
    color::Color,
    error::BoardError,
    piece::Piece,
    Board,
};
use common::bitboard::{
    bitboard::Bitboard,
    square::{self, *},
};

use super::{pawn_promotion::PawnPromotionChessMove, Capture};

#[derive(PartialEq, Clone, Eq, PartialOrd, Ord)]
pub struct StandardChessMove {
    from_square: Bitboard,
    to_square: Bitboard,
    capture: Option<Capture>,
}

impl StandardChessMove {
    pub fn new(from_square: Bitboard, to_square: Bitboard, capture: Option<Capture>) -> Self {
        Self {
            from_square,
            to_square,
            capture,
        }
    }

    pub fn to_square(&self) -> Bitboard {
        self.to_square
    }

    pub fn from_square(&self) -> Bitboard {
        self.from_square
    }

    pub fn capture(&self) -> Option<Capture> {
        self.capture
    }

    pub fn apply(&self, board: &mut Board) -> Result<Option<Capture>, BoardError> {
        let StandardChessMove {
            from_square,
            to_square,
            capture: expected_capture,
        } = self;

        let (piece_to_move, color) = board
            .remove(*from_square)
            .ok_or(BoardError::FromSquareIsEmpty { op: "apply" })?;
        if board.get(*to_square) != *expected_capture {
            return Err(BoardError::UnexpectedCaptureResult);
        }
        let captured_piece = board.remove(*to_square);

        let en_passant_target =
            get_en_passant_target_square(piece_to_move, color, *from_square, *to_square);
        let lost_castle_rights =
            get_lost_castle_rights_if_rook_or_king_moved(piece_to_move, color, *from_square)
                | get_lost_castle_rights_if_rook_taken(captured_piece, *to_square);

        if captured_piece.is_some() {
            board.reset_halfmove_clock();
        } else {
            board.increment_halfmove_clock();
        }
        board.increment_fullmove_clock();
        board.push_en_passant_target(en_passant_target);
        board.lose_castle_rights(lost_castle_rights);
        board.put(*to_square, piece_to_move, color).unwrap();

        Ok(captured_piece)
    }

    pub fn undo(&self, board: &mut Board) -> Result<Option<Capture>, BoardError> {
        let StandardChessMove {
            from_square,
            to_square,
            capture,
        } = self;

        // Remove the moved piece.
        let (piece_to_move_back, piece_color) = board
            .remove(*to_square)
            .ok_or(BoardError::ToSquareIsEmpty { op: "undo" })?;

        // Put the captured piece back.
        if capture.is_some() {
            let (piece, color) = capture.unwrap();
            board.put(*to_square, piece, color).unwrap();
        }

        // Revert the board state.
        board.pop_halfmove_clock();
        board.decrement_fullmove_clock();
        board.pop_en_passant_target();
        board.pop_castle_rights();
        board
            .put(*from_square, piece_to_move_back, piece_color)
            .unwrap();
        Ok(None)
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
            return Err(BoardError::PawnNotPromotable);
        }

        Ok(PawnPromotionChessMove::new(
            self.from_square,
            self.to_square,
            self.capture,
            promote_to_piece,
        ))
    }
}

/// Determines if a move is an en passant move. If so, it returns the target square.
/// Otherwise, it returns an empty square.
fn get_en_passant_target_square(
    piece_to_move: Piece,
    color: Color,
    from_square: Bitboard,
    to_square: Bitboard,
) -> Bitboard {
    if piece_to_move != Piece::Pawn {
        return Bitboard::EMPTY;
    }

    let is_en_passant = match color {
        Color::White => {
            from_square.overlaps(Bitboard::RANK_2) && to_square.overlaps(Bitboard::RANK_4)
        }
        Color::Black => {
            from_square.overlaps(Bitboard::RANK_7) && to_square.overlaps(Bitboard::RANK_5)
        }
    };

    if !is_en_passant {
        return Bitboard::EMPTY;
    }

    match color {
        Color::White => from_square << 8,
        Color::Black => from_square >> 8,
    }
}

fn get_lost_castle_rights_if_rook_or_king_moved(
    piece_to_move: Piece,
    color: Color,
    from_square: Bitboard,
) -> u8 {
    match (piece_to_move, color, from_square) {
        (Piece::Rook, Color::White, A1) => WHITE_QUEENSIDE_RIGHTS,
        (Piece::Rook, Color::White, H1) => WHITE_KINGSIDE_RIGHTS,
        (Piece::Rook, Color::Black, A8) => BLACK_QUEENSIDE_RIGHTS,
        (Piece::Rook, Color::Black, H8) => BLACK_KINGSIDE_RIGHTS,
        (Piece::King, Color::White, E1) => WHITE_KINGSIDE_RIGHTS | WHITE_QUEENSIDE_RIGHTS,
        (Piece::King, Color::Black, E8) => BLACK_KINGSIDE_RIGHTS | BLACK_QUEENSIDE_RIGHTS,
        _ => 0,
    }
}

fn get_lost_castle_rights_if_rook_taken(
    captured_piece: Option<(Piece, Color)>,
    to_square: Bitboard,
) -> u8 {
    match (captured_piece, to_square) {
        (Some((Piece::Rook, Color::White)), A1) => WHITE_QUEENSIDE_RIGHTS,
        (Some((Piece::Rook, Color::White)), H1) => WHITE_KINGSIDE_RIGHTS,
        (Some((Piece::Rook, Color::Black)), A8) => BLACK_QUEENSIDE_RIGHTS,
        (Some((Piece::Rook, Color::Black)), H8) => BLACK_KINGSIDE_RIGHTS,
        _ => 0,
    }
}

impl fmt::Display for StandardChessMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let capture_msg = match self.capture {
            Some((piece, color)) => format!(" (captures {})", piece.to_fen(color)),
            None => "".to_string(),
        };

        write!(
            f,
            "move {}{}{}",
            square::to_algebraic(self.from_square).to_lowercase(),
            square::to_algebraic(self.to_square).to_lowercase(),
            capture_msg
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
    ($from:expr, $to:expr, $capture:expr) => {
        ChessMove::Standard(StandardChessMove::new($from, $to, Some($capture)))
    };
    ($from:expr, $to:expr) => {
        ChessMove::Standard(StandardChessMove::new($from, $to, None))
    };
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::chess_move::ChessMove;

    #[test]
    fn test_apply_chess_move() {
        let mut board = Board::starting_position();
        println!("Testing board:\n{}", board);

        // using a queens gambit accepted opening to test basic chess move application
        let moves = vec![
            (E2, E4, (Piece::Pawn, Color::White), None),
            (E7, E5, (Piece::Pawn, Color::Black), None),
            (D2, D4, (Piece::Pawn, Color::White), None),
            (
                E5,
                D4,
                (Piece::Pawn, Color::Black),
                Some((Piece::Pawn, Color::White)),
            ),
        ];

        for (from_square, to_square, moved, expected_capture) in &moves {
            let chess_move = StandardChessMove::new(*from_square, *to_square, *expected_capture);
            let captured = chess_move.apply(&mut board).unwrap();
            assert_eq!(board.get(*to_square).unwrap(), *moved);
            assert_eq!(captured, *expected_capture);
            println!("New board state:\n{}", board);
        }
    }

    #[test]
    fn test_undo_pawn_move() {
        let mut board = Board::starting_position();
        let original_board = format!("{}", board);
        println!("Testing board:\n{}", board);

        let chess_move = std_move!(A2, A4);
        chess_move.apply(&mut board).unwrap();
        println!("Result after applying move:\n{}", board);
        chess_move.undo(&mut board).unwrap();
        println!("Result after undoing move:\n{}", board);

        let result_board = format!("{}", board);
        assert_eq!(original_board, result_board);
    }

    #[test]
    fn test_undo_knight_move() {
        let mut board = Board::starting_position();
        let original_board = format!("{}", board);
        println!("Testing board:\n{}", board);

        let chess_move = std_move!(B1, C3);
        chess_move.apply(&mut board).unwrap();
        println!("Result after applying move:\n{}", board);
        chess_move.undo(&mut board).unwrap();
        println!("Result after undoing move:\n{}", board);

        let chess_move_2 = std_move!(G1, F3);
        chess_move_2.apply(&mut board).unwrap();
        println!("Result after applying move:\n{}", board);
        chess_move_2.undo(&mut board).unwrap();
        println!("Result after undoing move:\n{}", board);

        let result_board = format!("{}", board);
        assert_eq!(original_board, result_board);
    }

    #[test]
    fn test_undo_capture() {
        let mut board = Board::new();
        board.put(A2, Piece::Knight, Color::White).unwrap();
        board.put(B4, Piece::Pawn, Color::Black).unwrap();
        let capture = std_move!(A2, B4, (Piece::Pawn, Color::Black));

        capture.apply(&mut board).unwrap();
        capture.undo(&mut board).unwrap();

        assert_eq!(Some((Piece::Knight, Color::White)), board.get(A2));
        assert_eq!(Some((Piece::Pawn, Color::Black)), board.get(B4));
    }

    #[test]
    fn test_white_lose_kingside_castle_rights() {
        let mut board = Board::new();
        board.put(E1, Piece::King, Color::White).unwrap();
        board.put(H1, Piece::Rook, Color::White).unwrap();
        println!("Testing board:\n{}", board);

        assert!(board.peek_castle_rights() & WHITE_KINGSIDE_RIGHTS > 0);
        let chess_move = std_move!(H1, H2);
        chess_move.apply(&mut board).unwrap();
        assert_eq!(0, board.peek_castle_rights() & WHITE_KINGSIDE_RIGHTS);
    }

    #[test]
    fn test_white_lose_queenside_castle_rights() {
        let mut board = Board::new();
        board.put(E1, Piece::King, Color::White).unwrap();
        board.put(A1, Piece::Rook, Color::White).unwrap();
        println!("Testing board:\n{}", board);

        assert!(board.peek_castle_rights() & WHITE_QUEENSIDE_RIGHTS > 0);
        let chess_move = std_move!(A1, A2);
        chess_move.apply(&mut board).unwrap();
        assert_eq!(0, board.peek_castle_rights() & WHITE_QUEENSIDE_RIGHTS);
    }

    #[test]
    fn test_black_lose_kingside_castle_rights() {
        let mut board = Board::new();
        board.put(E8, Piece::King, Color::Black).unwrap();
        board.put(H8, Piece::Rook, Color::Black).unwrap();
        println!("Testing board:\n{}", board);

        assert!(board.peek_castle_rights() & BLACK_KINGSIDE_RIGHTS > 0);
        let chess_move = std_move!(H8, H2);
        chess_move.apply(&mut board).unwrap();
        assert_eq!(0, board.peek_castle_rights() & BLACK_KINGSIDE_RIGHTS);
    }

    #[test]
    fn test_black_lose_queenside_castle_rights() {
        let mut board = Board::new();
        board.put(E8, Piece::King, Color::Black).unwrap();
        board.put(A8, Piece::Rook, Color::Black).unwrap();
        println!("Testing board:\n{}", board);

        assert!(board.peek_castle_rights() & BLACK_QUEENSIDE_RIGHTS > 0);
        let chess_move = std_move!(A8, A2);
        chess_move.apply(&mut board).unwrap();
        assert_eq!(0, board.peek_castle_rights() & BLACK_QUEENSIDE_RIGHTS);
    }

    #[test]
    fn test_white_lose_queenside_castle_rights_from_capture() {
        let mut board = Board::new();
        board.put(E1, Piece::King, Color::White).unwrap();
        board.put(A1, Piece::Rook, Color::White).unwrap();
        board.put(H1, Piece::Rook, Color::White).unwrap();
        board.put(H8, Piece::Bishop, Color::Black).unwrap();
        println!("Testing board:\n{}", board);

        assert!(board.peek_castle_rights() & WHITE_QUEENSIDE_RIGHTS > 0);
        let chess_move = std_move!(H8, A1, (Piece::Rook, Color::White));
        chess_move.apply(&mut board).unwrap();
        assert_eq!(0, board.peek_castle_rights() & WHITE_QUEENSIDE_RIGHTS);
    }

    #[test]
    fn test_white_lose_kingside_castle_rights_from_capture() {
        let mut board = Board::new();
        board.put(E1, Piece::King, Color::White).unwrap();
        board.put(A1, Piece::Rook, Color::White).unwrap();
        board.put(H1, Piece::Rook, Color::White).unwrap();
        board.put(A8, Piece::Bishop, Color::Black).unwrap();
        println!("Testing board:\n{}", board);

        assert!(board.peek_castle_rights() & WHITE_KINGSIDE_RIGHTS > 0);
        let chess_move = std_move!(A8, H1, (Piece::Rook, Color::White));
        chess_move.apply(&mut board).unwrap();
        assert_eq!(0, board.peek_castle_rights() & WHITE_KINGSIDE_RIGHTS);
    }

    #[test]
    fn test_black_lose_queenside_castle_rights_from_capture() {
        let mut board = Board::new();
        board.put(E8, Piece::King, Color::Black).unwrap();
        board.put(A8, Piece::Rook, Color::Black).unwrap();
        board.put(H8, Piece::Rook, Color::Black).unwrap();
        board.put(H1, Piece::Bishop, Color::White).unwrap();
        println!("Testing board:\n{}", board);

        assert!(board.peek_castle_rights() & BLACK_QUEENSIDE_RIGHTS > 0);
        let chess_move = std_move!(H1, A8, (Piece::Rook, Color::Black));
        chess_move.apply(&mut board).unwrap();
        assert_eq!(0, board.peek_castle_rights() & BLACK_QUEENSIDE_RIGHTS);
    }

    #[test]
    fn test_black_lose_kingside_castle_rights_from_capture() {
        let mut board = Board::new();
        board.put(E8, Piece::King, Color::Black).unwrap();
        board.put(A8, Piece::Rook, Color::Black).unwrap();
        board.put(H8, Piece::Rook, Color::Black).unwrap();
        board.put(A1, Piece::Bishop, Color::White).unwrap();
        println!("Testing board:\n{}", board);

        assert!(board.peek_castle_rights() & BLACK_KINGSIDE_RIGHTS > 0);
        let chess_move = std_move!(A1, H8, (Piece::Rook, Color::Black));
        chess_move.apply(&mut board).unwrap();
        assert_eq!(0, board.peek_castle_rights() & BLACK_KINGSIDE_RIGHTS);
    }

    #[test]
    fn test_white_lose_all_castle_rights() {
        let mut board = Board::new();
        board.put(E1, Piece::King, Color::White).unwrap();
        board.put(A1, Piece::Rook, Color::White).unwrap();
        board.put(H1, Piece::Rook, Color::White).unwrap();
        println!("Testing board:\n{}", board);

        assert!(board.peek_castle_rights() & WHITE_KINGSIDE_RIGHTS > 0);
        assert!(board.peek_castle_rights() & WHITE_QUEENSIDE_RIGHTS > 0);
        let chess_move = std_move!(E1, E2);
        chess_move.apply(&mut board).unwrap();
        assert_eq!(0, board.peek_castle_rights() & WHITE_KINGSIDE_RIGHTS);
        assert_eq!(0, board.peek_castle_rights() & WHITE_QUEENSIDE_RIGHTS);
    }

    #[test]
    fn test_black_lose_all_castle_rights() {
        let mut board = Board::new();
        board.put(E8, Piece::King, Color::Black).unwrap();
        board.put(A8, Piece::Rook, Color::Black).unwrap();
        board.put(H8, Piece::Rook, Color::Black).unwrap();
        println!("Testing board:\n{}", board);

        assert!(board.peek_castle_rights() & BLACK_KINGSIDE_RIGHTS > 0);
        assert!(board.peek_castle_rights() & BLACK_QUEENSIDE_RIGHTS > 0);
        let chess_move = std_move!(E8, E7);
        chess_move.apply(&mut board).unwrap();
        assert_eq!(0, board.peek_castle_rights() & BLACK_KINGSIDE_RIGHTS);
        assert_eq!(0, board.peek_castle_rights() & BLACK_QUEENSIDE_RIGHTS);
    }

    #[test]
    fn test_zobrist_hashing_reversible_for_standard_move() {
        let mut board = Board::new();
        board.put(E2, Piece::Pawn, Color::White).unwrap();
        let initial_hash = board.current_position_hash();

        let chess_move = std_move!(E2, E4);
        chess_move.apply(&mut board).unwrap();
        assert_ne!(initial_hash, board.current_position_hash());

        chess_move.undo(&mut board).unwrap();
        assert_eq!(initial_hash, board.current_position_hash());
    }
}
