use super::ChessMove;
use crate::board::color::Color;
use crate::board::piece::Piece;
use crate::board::Board;

impl Board {
    /// Applies a chess move to the board. If this resulted in a capture,
    /// the captured piece is returned.
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::square::Square;

    #[test]
    fn test_apply_chess_move() {
        let mut board = Board::starting_position();
        println!("Testing board:\n{}", board.to_ascii());

        // using a queens gambit accepted opening to test basic chess move application
        let moves: Vec<(Square, Square, (Piece, Color), Option<(Piece, Color)>)> = vec![
            (Square::E2, Square::E4, (Piece::Pawn, Color::White), None),
            (Square::E7, Square::E5, (Piece::Pawn, Color::Black), None),
            (Square::D2, Square::D4, (Piece::Pawn, Color::White), None),
            (
                Square::E5,
                Square::D4,
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
}
