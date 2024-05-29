use common::bitboard::square::from_rank_file;

use super::color::Color;
use super::piece::Piece;
use super::Board;
use std::fmt;

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let divider = "+---+---+---+---+---+---+---+---+";
        let files: [char; 8] = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H'];
        let ranks: [char; 8] = ['1', '2', '3', '4', '5', '6', '7', '8'];

        let mut rows: Vec<String> = vec![];

        for rank in (0..8).rev() {
            let mut cells: Vec<String> = vec![];
            for file in 0..8 {
                let sq = from_rank_file(rank, file);
                let cell = match self.get(sq) {
                    Some((piece, color)) => get_piece_char(piece, color),
                    None => ' ',
                };
                cells.push(cell.to_string());
            }
            let formatted_cells = format!("| {} |", cells.join(" | "));

            rows.push(format!("{} {}", ' ', divider));
            rows.push(format!("{} {}", ranks[rank as usize], formatted_cells));
        }
        rows.push(format!("{} {}", ' ', divider));
        let formatted_ranks_footer = format!(
            "  {}  ",
            files
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<String>>()
                .join("   ")
        );
        rows.push(format!("{} {}", ' ', formatted_ranks_footer));

        write!(f, "{}", rows.join("\n"))
    }
}

fn get_piece_char(piece: Piece, color: Color) -> char {
    match (piece, color) {
        (Piece::Bishop, Color::Black) => '♗',
        (Piece::Bishop, Color::White) => '♝',
        (Piece::King, Color::Black) => '♔',
        (Piece::King, Color::White) => '♚',
        (Piece::Knight, Color::Black) => '♘',
        (Piece::Knight, Color::White) => '♞',
        (Piece::Pawn, Color::Black) => '♙',
        (Piece::Pawn, Color::White) => '♟',
        (Piece::Queen, Color::Black) => '♕',
        (Piece::Queen, Color::White) => '♛',
        (Piece::Rook, Color::Black) => '♖',
        (Piece::Rook, Color::White) => '♜',
    }
}
