use core::fmt;

use crate::board::{
    bitboard::{EMPTY, RANK_1, RANK_2, RANK_4, RANK_5, RANK_7, RANK_8},
    color::Color,
    error::BoardError,
    piece::Piece,
    square::{self, A1, A8, E1, E8, H1, H8},
    Board, BLACK_KINGSIDE_RIGHTS, BLACK_QUEENSIDE_RIGHTS, WHITE_KINGSIDE_RIGHTS,
    WHITE_QUEENSIDE_RIGHTS,
};

use super::{pawn_promotion::PawnPromotionChessMove, Capture, ChessMove};

#[derive(PartialEq, Clone)]
pub struct StandardChessMove {
    from_square: u64,
    to_square: u64,
    capture: Option<Capture>,
}

impl StandardChessMove {
    pub fn new(from_square: u64, to_square: u64, capture: Option<Capture>) -> Self {
        Self {
            from_square,
            to_square,
            capture,
        }
    }

    // In move generation, we convert standard pawn moves on the final rank
    // to pawn promotion moves. These are helpers used in the move generation.

    pub fn is_promotable_pawn(&self, board: &Board) -> bool {
        let pawn_color = match board.get(self.from_square) {
            Some((Piece::Pawn, color)) => color,
            _ => return false,
        };
        let (from_rank, to_rank) = match pawn_color {
            Color::White => (self.from_square & RANK_7, self.to_square & RANK_8),
            Color::Black => (self.from_square & RANK_2, self.to_square & RANK_1),
        };

        from_rank > 0 && to_rank > 0
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

impl ChessMove for StandardChessMove {
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
        let StandardChessMove {
            from_square,
            to_square,
            capture: expected_capture,
        } = self;

        let maybe_piece = board.remove(*from_square);
        let (piece_to_move, color) = match maybe_piece {
            None => return Err(BoardError::FromSquareIsEmpty { op: "apply" }),
            Some((piece, color)) => (piece, color),
        };

        if board.get(*to_square) != *expected_capture {
            return Err(BoardError::UnexpectedCaptureResult);
        }

        // check for en passant
        let mut en_passant_target = EMPTY;
        if piece_to_move == Piece::Pawn {
            let is_en_passant = match color {
                Color::White => (from_square & RANK_2 > 0) && (to_square & RANK_4 > 0),
                Color::Black => (from_square & RANK_7 > 0) && (to_square & RANK_5 > 0),
            };

            if is_en_passant {
                en_passant_target = match color {
                    Color::White => from_square << 8,
                    Color::Black => from_square >> 8,
                };
            }
        }
        board.push_en_passant_target(en_passant_target);

        let captured_piece = board.remove(*to_square);

        // adjust castle rights if a rook or king moved
        let mut lost_castle_rights = match (piece_to_move, color, *from_square) {
            (Piece::Rook, Color::White, A1) => WHITE_QUEENSIDE_RIGHTS,
            (Piece::Rook, Color::White, H1) => WHITE_KINGSIDE_RIGHTS,
            (Piece::Rook, Color::Black, A8) => BLACK_QUEENSIDE_RIGHTS,
            (Piece::Rook, Color::Black, H8) => BLACK_KINGSIDE_RIGHTS,
            (Piece::King, Color::White, E1) => WHITE_KINGSIDE_RIGHTS | WHITE_QUEENSIDE_RIGHTS,
            (Piece::King, Color::Black, E8) => BLACK_KINGSIDE_RIGHTS | BLACK_QUEENSIDE_RIGHTS,
            _ => 0,
        };

        // adjust castle rights if a rook is taken
        lost_castle_rights |= match (captured_piece, *to_square) {
            (Some((Piece::Rook, Color::White)), A1) => WHITE_QUEENSIDE_RIGHTS,
            (Some((Piece::Rook, Color::White)), H1) => WHITE_KINGSIDE_RIGHTS,
            (Some((Piece::Rook, Color::Black)), A8) => BLACK_QUEENSIDE_RIGHTS,
            (Some((Piece::Rook, Color::Black)), H8) => BLACK_KINGSIDE_RIGHTS,
            _ => 0,
        };

        board.lose_castle_rights(lost_castle_rights);

        board
            .put(*to_square, piece_to_move, color)
            .map(|_| captured_piece)
    }

    fn undo(&self, board: &mut Board) -> Result<Option<Capture>, BoardError> {
        let StandardChessMove {
            from_square,
            to_square,
            capture,
        } = self;

        // remove the moved piece
        let maybe_piece = board.remove(*to_square);
        let (piece_to_move_back, piece_color) = match maybe_piece {
            None => return Err(BoardError::ToSquareIsEmpty { op: "undo" }),
            Some((piece, color)) => (piece, color),
        };

        // put the captured piece back
        if capture.is_some() {
            let (piece, color) = capture.unwrap();
            board.put(*to_square, piece, color).unwrap();
        }

        // return to the previous en passant state
        board.pop_en_passant_target();

        // return to the previous castle rights state
        board.pop_castle_rights();

        board
            .put(*from_square, piece_to_move_back, piece_color)
            .map(|_| None)
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
        StandardChessMove::new($from, $to, Some($capture))
    };
    ($from:expr, $to:expr) => {
        StandardChessMove::new($from, $to, None)
    };
}

#[cfg(test)]
mod tests {
    use crate::board::square::{A2, A4, B1, B4, C3, D2, D4, E2, E4, E5, E7, F3, G1, H2};

    use super::*;

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
}
