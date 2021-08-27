use super::ChessMove;
use crate::board::bitboard::{EMPTY, RANK_2, RANK_4, RANK_5, RANK_7};
use crate::board::color::Color;
use crate::board::piece::Piece;
use crate::board::Board;

type Capture = (Piece, Color);
type BoardMoveResult = Result<Option<Capture>, &'static str>;

impl Board {
    pub fn apply(&mut self, chessmove: ChessMove) -> BoardMoveResult {
        match chessmove {
            ChessMove::Basic {
                from_square,
                to_square,
                capture,
            } => self.apply_basic(from_square, to_square, capture),
            ChessMove::EnPassant {
                from_square,
                to_square,
            } => self.apply_en_passant(from_square, to_square),
        }
    }

    fn apply_basic(
        &mut self,
        from_square: u64,
        to_square: u64,
        expected_capture: Option<Capture>,
    ) -> BoardMoveResult {
        let maybe_piece = self.remove(from_square);
        let (piece_to_move, color) = match maybe_piece {
            None => return Err("cannot apply chess move, the `from` square is empty"),
            Some((piece, color)) => (piece, color),
        };

        if self.get(to_square) != expected_capture {
            return Err(
                "the expected capture result is different than what is on the target square",
            );
        }

        // check for en passant
        if piece_to_move == Piece::Pawn {
            let is_en_passant = match color {
                Color::White => (from_square & RANK_2 > 0) && (to_square & RANK_4 > 0),
                Color::Black => (from_square & RANK_7 > 0) && (to_square & RANK_5 > 0),
            };

            if is_en_passant {
                match color {
                    Color::White => self.push_en_passant_target(from_square << 8),
                    Color::Black => self.push_en_passant_target(from_square >> 8),
                };
            } else {
                self.push_en_passant_target(EMPTY);
            }
        } else {
            self.push_en_passant_target(EMPTY);
        }

        let captured_piece = self.remove(to_square);
        match self.put(to_square, piece_to_move, color) {
            Ok(()) => return Ok(captured_piece),
            Err(error) => return Err(error),
        }
    }

    fn apply_en_passant(&mut self, from_square: u64, to_square: u64) -> BoardMoveResult {
        let maybe_piece = self.remove(from_square);
        let (piece_to_move, color) = match maybe_piece {
            None => return Err("cannot apply chess move, the `from` square is empty"),
            Some((piece, color)) => (piece, color),
        };

        if piece_to_move != Piece::Pawn {
            return Err("cannot apply en passant, the piece is not a pawn");
        }

        // the captured pawn is "behind" the target square
        let capture_square = match color {
            Color::White => to_square >> 8,
            Color::Black => to_square << 8,
        };

        let capture = match self.remove(capture_square) {
            Some((piece, color)) => (piece, color),
            None => return Err("en passant didn't result in a capture"),
        };

        self.put(to_square, piece_to_move, color).unwrap();

        self.push_en_passant_target(EMPTY);

        Ok(Some(capture))
    }

    pub fn undo(&mut self, chessmove: ChessMove) -> BoardMoveResult {
        match chessmove {
            ChessMove::Basic {
                from_square,
                to_square,
                capture,
            } => self.undo_basic(from_square, to_square, capture),
            ChessMove::EnPassant {
                from_square,
                to_square,
            } => self.undo_en_passant(from_square, to_square),
        }
    }

    fn undo_basic(
        &mut self,
        from_square: u64,
        to_square: u64,
        capture: Option<Capture>,
    ) -> BoardMoveResult {
        // remove the moved piece
        let maybe_piece = self.remove(to_square);
        let (piece_to_move_back, piece_color) = match maybe_piece {
            None => return Err("cannot undo chess move, the `to` square is empty"),
            Some((piece, color)) => (piece, color),
        };

        // put the captured piece back
        if capture.is_some() {
            let (piece, color) = capture.unwrap();
            self.put(to_square, piece, color).unwrap();
        }

        // return to the previous en passant state
        self.pop_en_passant_target();

        match self.put(from_square, piece_to_move_back, piece_color) {
            Ok(()) => return Ok(None),
            Err(error) => return Err(error),
        }
    }

    fn undo_en_passant(&mut self, from_square: u64, to_square: u64) -> BoardMoveResult {
        // remove the moved pawn
        let maybe_piece = self.remove(to_square);
        let (piece_to_move_back, piece_color) = match maybe_piece {
            None => return Err("cannot undo chess move, the `to` square is empty"),
            Some((piece, color)) => (piece, color),
        };

        if piece_to_move_back != Piece::Pawn {
            return Err("cannot undo en passant, the piece is not a pawn");
        }

        // return the pawn to its original square
        self.put(from_square, piece_to_move_back, piece_color)
            .unwrap();

        // the captured pawn is "behind" the target square
        let capture_square = match piece_color {
            Color::White => to_square >> 8,
            Color::Black => to_square << 8,
        };
        self.put(capture_square, Piece::Pawn, piece_color.opposite())
            .unwrap();

        // return to the previous en passant state
        self.pop_en_passant_target();

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::square;

    #[test]
    fn test_apply_chess_move() {
        let mut board = Board::starting_position();
        println!("Testing board:\n{}", board.to_ascii());

        // using a queens gambit accepted opening to test basic chess move application
        let moves: Vec<(u64, u64, (Piece, Color), Option<(Piece, Color)>)> = vec![
            (square::E2, square::E4, (Piece::Pawn, Color::White), None),
            (square::E7, square::E5, (Piece::Pawn, Color::Black), None),
            (square::D2, square::D4, (Piece::Pawn, Color::White), None),
            (
                square::E5,
                square::D4,
                (Piece::Pawn, Color::Black),
                Some((Piece::Pawn, Color::White)),
            ),
        ];

        for (from_square, to_square, moved, expected_capture) in &moves {
            let captured = board
                .apply(ChessMove::basic(
                    *from_square,
                    *to_square,
                    *expected_capture,
                ))
                .unwrap();
            assert_eq!(board.get(*to_square).unwrap(), *moved);
            assert_eq!(captured, *expected_capture);
            println!("New board state:\n{}", board.to_ascii());
        }
    }

    #[test]
    fn test_undo_pawn_move() {
        let mut board = Board::starting_position();
        let original_board = board.to_ascii();
        println!("Testing board:\n{}", board.to_ascii());

        let chessmove = ChessMove::basic(square::A2, square::A4, None);
        board.apply(chessmove).unwrap();
        println!("Result after applying move:\n{}", board.to_ascii());
        board.undo(chessmove).unwrap();
        println!("Result after undoing move:\n{}", board.to_ascii());

        let result_board = board.to_ascii();
        assert_eq!(original_board, result_board);
    }

    #[test]
    fn test_undo_knight_move() {
        let mut board = Board::starting_position();
        let original_board = board.to_ascii();
        println!("Testing board:\n{}", board.to_ascii());

        let chessmove = ChessMove::basic(square::B1, square::C3, None);
        board.apply(chessmove).unwrap();
        println!("Result after applying move:\n{}", board.to_ascii());
        board.undo(chessmove).unwrap();
        println!("Result after undoing move:\n{}", board.to_ascii());

        let chessmove2 = ChessMove::basic(square::B1, square::A3, None);
        board.apply(chessmove2).unwrap();
        println!("Result after applying move:\n{}", board.to_ascii());
        board.undo(chessmove2).unwrap();
        println!("Result after undoing move:\n{}", board.to_ascii());

        let result_board = board.to_ascii();
        assert_eq!(original_board, result_board);
    }

    #[test]
    fn test_undo_capture() {
        let mut board = Board::new();
        board.put(square::A2, Piece::Knight, Color::White).unwrap();
        board.put(square::B4, Piece::Pawn, Color::Black).unwrap();
        let capture = ChessMove::basic(square::A2, square::B4, Some((Piece::Pawn, Color::Black)));

        board.apply(capture).unwrap();
        board.undo(capture).unwrap();

        assert_eq!(Some((Piece::Knight, Color::White)), board.get(square::A2));
        assert_eq!(Some((Piece::Pawn, Color::Black)), board.get(square::B4));
    }

    #[test]
    fn test_apply_and_undo_en_passant() {
        let mut board = Board::new();
        board.put(square::D2, Piece::Pawn, Color::White).unwrap();
        board.put(square::E4, Piece::Pawn, Color::Black).unwrap();
        println!("Testing board:\n{}", board.to_ascii());

        board
            .apply(ChessMove::basic(square::D2, square::D4, None))
            .unwrap();
        println!("After move that reveals en passant:\n{}", board.to_ascii());
        assert_eq!(Some((Piece::Pawn, Color::White)), board.get(square::D4));
        assert_eq!(square::D3, board.peek_en_passant_target());

        let en_passant = ChessMove::en_passant(square::E4, square::D3);

        board.apply(en_passant).unwrap();
        println!("After en passant:\n{}", board.to_ascii());
        assert_eq!(Some((Piece::Pawn, Color::Black)), board.get(square::D3));
        assert_eq!(None, board.get(square::D4));
        assert_eq!(EMPTY, board.peek_en_passant_target());

        board.undo(en_passant).unwrap();
        println!("Undo en passant:\n{}", board.to_ascii());
        assert_eq!(Some((Piece::Pawn, Color::White)), board.get(square::D4));
        assert_eq!(Some((Piece::Pawn, Color::Black)), board.get(square::E4));
        assert_eq!(square::D3, board.peek_en_passant_target());
    }
}
