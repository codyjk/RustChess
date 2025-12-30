use crate::game::display::GameDisplay;

use super::Board;
use std::fmt;

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut ui = GameDisplay::new();
        ui.render_game_state(self, self.turn(), None, None, None);
        write!(f, "{}", ui.buffer())
    }
}

#[macro_export]
macro_rules! chess_position {
    ($($piece:tt)*) => {{
        let mut board = Board::new();
        // Convert all input tokens to a string and filter out whitespace characters.
        let pieces: Vec<_> = stringify!($($piece)*)
            .chars()
            .filter(|&c| !c.is_whitespace())
            .collect();
        // Ensure we have exactly 64 squares
        assert_eq!(pieces.len(), 64, "Invalid number of squares. Expected 64, got {}", pieces.len());
        // Iterate over the remaining characters and set up the board.
        for (i, &c) in pieces.iter().enumerate().rev() {
            if c != '.' {
                // Map character to corresponding piece and color.
                let (piece, color) = match c {
                    'K' => (Piece::King, Color::White),
                    'Q' => (Piece::Queen, Color::White),
                    'R' => (Piece::Rook, Color::White),
                    'B' => (Piece::Bishop, Color::White),
                    'N' => (Piece::Knight, Color::White),
                    'P' => (Piece::Pawn, Color::White),
                    'k' => (Piece::King, Color::Black),
                    'q' => (Piece::Queen, Color::Black),
                    'r' => (Piece::Rook, Color::Black),
                    'b' => (Piece::Bishop, Color::Black),
                    'n' => (Piece::Knight, Color::Black),
                    'p' => (Piece::Pawn, Color::Black),
                    _ => panic!("Invalid character in chess position"),
                };
                // The macro input is from white's perspective, so the bottom
                // left character is A1 (rather than the first character in the
                // sequence). So we need to transpose the sequence back to the
                // correct order.
                let row = i / 8;
                let col = i % 8;
                let transposed_row = 7 - row;
                let square_num = (8 * transposed_row + col) as u8;
                board.put(common::bitboard::square::Square::new(square_num), piece, color).unwrap();
            }
        }
        board
    }};
}
