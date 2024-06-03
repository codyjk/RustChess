use common::bitboard::square::from_rank_file;

use super::color::Color;
use super::piece::Piece;
use super::Board;
use std::fmt;

const EMPTY_CELL: char = '.';

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut rows: Vec<String> = vec![];

        for rank in (0..8).rev() {
            let mut cells: Vec<String> = vec![];
            for file in 0..8 {
                let sq = from_rank_file(rank, file);
                let cell = match self.get(sq) {
                    Some((piece, color)) => get_piece_char(piece, color),
                    None => EMPTY_CELL,
                };
                cells.push(cell.to_string());
            }
            rows.push(cells.join("").to_string());
        }
        write!(f, "{}", rows.join("\n"))
    }
}

fn get_piece_char(piece: Piece, color: Color) -> char {
    match (piece, color, cfg!(test)) {
        // In the test environment, we want the printed board to match
        // what is passed in to the `chess_position!` macro, to make it easy
        // to copy-paste the expected board state into the test.
        (Piece::Bishop, Color::Black, true) => 'b',
        (Piece::Bishop, Color::White, true) => 'B',
        (Piece::King, Color::Black, true) => 'k',
        (Piece::King, Color::White, true) => 'K',
        (Piece::Knight, Color::Black, true) => 'n',
        (Piece::Knight, Color::White, true) => 'N',
        (Piece::Pawn, Color::Black, true) => 'p',
        (Piece::Pawn, Color::White, true) => 'P',
        (Piece::Queen, Color::Black, true) => 'q',
        (Piece::Queen, Color::White, true) => 'Q',
        (Piece::Rook, Color::Black, true) => 'r',
        (Piece::Rook, Color::White, true) => 'R',
        // In the actual game, we want to use unicode characters to represent
        // the pieces, making it easier to visually distinguish them.
        (Piece::Bishop, Color::Black, false) => '♗',
        (Piece::Bishop, Color::White, false) => '♝',
        (Piece::King, Color::Black, false) => '♔',
        (Piece::King, Color::White, false) => '♚',
        (Piece::Knight, Color::Black, false) => '♘',
        (Piece::Knight, Color::White, false) => '♞',
        (Piece::Pawn, Color::Black, false) => '♙',
        (Piece::Pawn, Color::White, false) => '♟',
        (Piece::Queen, Color::Black, false) => '♕',
        (Piece::Queen, Color::White, false) => '♛',
        (Piece::Rook, Color::Black, false) => '♖',
        (Piece::Rook, Color::White, false) => '♜',
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
                let square_num = 8 * transposed_row + col;
                board.put(Bitboard(1 << square_num), piece, color).unwrap();
            }
        }
        board
    }};
}
