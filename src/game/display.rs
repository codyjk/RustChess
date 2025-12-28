use crate::board::{color::Color, Board};
use crate::chess_move::chess_move::ChessMove;
use common::bitboard::Square;
use std::fmt::Write;
use termion::{clear, cursor};

pub struct GameDisplay {
    buffer: String,
}

impl GameDisplay {
    pub fn new() -> Self {
        Self {
            buffer: String::with_capacity(2048),
        }
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        write!(self.buffer, "{}{}", cursor::Goto(1, 1), clear::All).unwrap();
    }

    pub fn render_game_state(
        &mut self,
        board: &Board,
        current_turn: Color,
        last_move: Option<(&ChessMove, &str)>,
        stats: Option<&str>,
    ) {
        self.clear();

        // Board header
        self.buffer.push_str("    a   b   c   d   e   f   g   h\n");
        self.buffer
            .push_str("  ┌───┬───┬───┬───┬───┬───┬───┬───┐\n");

        // Board squares
        for rank in (0..8u8).rev() {
            self.buffer.push_str(&format!("{} │", rank + 1));
            for file in 0..8u8 {
                let square = Square::from_rank_file(rank, file);
                let piece_str = match board.get(square) {
                    Some((piece, color)) => piece.to_unicode_piece_char(color).to_string(),
                    None => if (rank + file) % 2 == 0 { " " } else { "·" }.to_string(),
                };
                self.buffer.push_str(&format!(" {} │", piece_str));
            }
            self.buffer.push_str(&format!(" {}\n", rank + 1));

            if rank > 0 {
                self.buffer
                    .push_str("  ├───┼───┼───┼───┼───┼───┼───┼───┤\n");
            } else {
                self.buffer
                    .push_str("  └───┴───┴───┴───┴───┴───┴───┴───┘\n");
            }
        }

        // Board footer
        self.buffer
            .push_str("    a   b   c   d   e   f   g   h\n\n");

        // Game info
        self.buffer.push_str(&format!("Turn: {}\n", current_turn));

        if let Some((_mv, notation)) = last_move {
            self.buffer.push_str(&format!("Last move: {}\n", notation));
        }

        if let Some(stats) = stats {
            self.buffer.push_str(&format!("\n{}\n", stats));
        }

        // Print the complete frame
        print!("{}", self.buffer);
    }

    pub fn buffer(self) -> String {
        self.buffer
    }
}
