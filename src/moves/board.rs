use super::ChessMove;
use crate::board::color::Color;
use crate::board::piece::Piece;
use crate::board::Board;

impl Board {
    pub fn apply(&mut self, chessmove: ChessMove) -> Result<Option<(Piece, Color)>, &'static str> {
        let maybe_piece = self.remove(chessmove.from_square);
        let (piece_to_move, color) = match maybe_piece {
            None => return Err("cannot apply chess move, the `from` square is empty"),
            Some((piece, color)) => (piece, color),
        };

        let captured_piece = self.remove(chessmove.to_square);
        match self.put(chessmove.to_square, piece_to_move, color) {
            Ok(()) => return Ok(captured_piece),
            Err(error) => return Err(error),
        }
    }

    pub fn undo(&mut self, chessmove: ChessMove) -> Result<(), &'static str> {
        // remove the moved piece
        let maybe_piece = self.remove(chessmove.to_square);
        let (piece_to_move_back, piece_color) = match maybe_piece {
            None => return Err("cannot undo chess move, the `to` square is empty"),
            Some((piece, color)) => (piece, color),
        };

        // put the captured piece back
        if chessmove.capture.is_some() {
            let (piece, color) = chessmove.capture.unwrap();
            self.put(chessmove.to_square, piece, color).unwrap();
        }

        match self.put(chessmove.from_square, piece_to_move_back, piece_color) {
            Ok(()) => return Ok(()),
            Err(error) => return Err(error),
        }
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
                .apply(ChessMove {
                    from_square: *from_square,
                    to_square: *to_square,
                    capture: *expected_capture,
                })
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

        let chessmove = ChessMove::new(square::A2, square::A4, None);
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

        let chessmove = ChessMove::new(square::B1, square::C3, None);
        board.apply(chessmove).unwrap();
        println!("Result after applying move:\n{}", board.to_ascii());
        board.undo(chessmove).unwrap();
        println!("Result after undoing move:\n{}", board.to_ascii());

        let chessmove2 = ChessMove::new(square::B1, square::A3, None);
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
        let capture = ChessMove::new(square::A2, square::B4, Some((Piece::Pawn, Color::Black)));

        board.apply(capture).unwrap();
        board.undo(capture).unwrap();

        assert_eq!(Some((Piece::Knight, Color::White)), board.get(square::A2));
        assert_eq!(Some((Piece::Pawn, Color::Black)), board.get(square::B4));
    }
}
