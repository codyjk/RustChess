pub mod coordinate;
pub mod piece;

pub use coordinate::Coordinate;

use piece::Piece;
use regex::Regex;

pub struct Board {
    squares: Vec<Vec<Square>>,
}

struct Square {
    piece: Option<Piece>,
}

pub struct ChessMove {
    pub from_coord: Coordinate,
    pub to_coord: Coordinate,
}

impl ChessMove {
    pub fn from_algebraic(algebraic_move: String) -> Result<Self, &'static str> {
        let re = Regex::new("^([a-h][1-8])([a-h][1-8])$").unwrap();
        let caps = match re.captures(&algebraic_move) {
            Some(captures) => captures,
            None => return Err("invalid move"),
        };

        Ok(Self {
            from_coord: Coordinate::from_algebraic(&caps[1]),
            to_coord: Coordinate::from_algebraic(&caps[2]),
        })
    }
}

impl Board {
    /// Instantiates an empty board.
    pub fn new() -> Board {
        let squares = (0..8)
            .map(|_row| (0..8).map(|_col| Square { piece: None }).collect())
            .collect();

        Board { squares }
    }

    /// Shows the piece on the requested `coord`.
    pub fn get(&self, coord: Coordinate) -> Option<Piece> {
        let (row, col) = coord.to_row_col();
        self.squares[row][col].piece
    }

    /// Puts a `piece` on the requested `coord`
    pub fn put(&mut self, coord: Coordinate, piece: Option<Piece>) -> Option<Piece> {
        let (row, col) = coord.to_row_col();
        let prev = self.squares[row][col].piece;
        self.squares[row][col].piece = piece;
        prev
    }

    /// Applies a chess move to the board. If this resulted in a capture,
    /// the captured piece is returned.
    pub fn apply(&mut self, chessmove: &ChessMove) -> Result<Option<Piece>, &'static str> {
        match self.get(chessmove.from_coord) {
            None => return Err("cannot apply chess move, the `from` square is empty"),
            _ => (),
        };
        let piece_to_move = self.put(chessmove.from_coord, None);
        let captured_piece = self.put(chessmove.to_coord, piece_to_move);
        Ok(captured_piece)
    }

    /// A FEN record contains six fields. The separator between fields is a space. The fields are:
    ///   1. Piece placement (from White's perspective). Each rank is described, starting with rank 8
    ///     and ending with rank 1; within each rank, the contents of each square are described from
    ///     file `a` through file `h`. Following the Standard Algebraic Notation (SAN), each piece is
    ///     identified by a single letter taken from the standard English names (pawn = `P`,
    ///     knight = `N`, bishop = `B`, rook = `R`, queen = `Q` and king = `K`). White pieces are
    ///     designated using upper-case letters (`PNBRQK`) while black pieces use lowercase
    ///     (`pnbrqk`). Empty squares are noted using digits 1 through 8 (the number of empty
    ///     squares), and `/` separates ranks.
    ///   2. Active color. `w` means White moves next, `b` means Black moves next.
    ///   3. Castling availability. If neither side can castle, this is `-`. Otherwise, this has one
    ///     or more letters: `K` (White can castle kingside), `Q` (White can castle queenside), `k`
    ///     (Black can castle kingside), and/or `q` (Black can castle queenside). A move that
    ///     temporarily prevents castling does not negate this notation.
    ///   4. En passant target square in algebraic notation. If there's no en passant target square,
    ///     this is `-`. If a pawn has just made a two-square move, this is the position `behind` the
    ///     pawn. This is recorded regardless of whether there is a pawn in position to make an en
    ///     passant capture.
    ///   5. Halfmove clock: The number of halfmoves since the last capture or pawn advance, used for
    ///     the fifty-move rule.
    ///   6. Fullmove number: The number of the full move. It starts at 1, and is incremented after
    ///     Black's move.
    ///
    /// Starting position FEN: `rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1`
    pub fn from_fen(fen: &str) -> Result<Board, String> {
        let rank_partial_regex = "([pnbrqkPNBRQK1-8]{1,8})";
        let color_partial_regex = "(b|w)";
        let castling_partial_regex = "([kqKQ]{1,4}|-)";
        let en_passant_partial_regex = "([a-h][1-8]|-)";
        let halfmove_partial_regex = "(0|[1-9][0-9]*)";
        let fullmove_partial_regex = "([1-9][0-9]*)";
        let full_fen_regex = format!(
            "^{}/{}/{}/{}/{}/{}/{}/{} {} {} {} {} {}$",
            rank_partial_regex,
            rank_partial_regex,
            rank_partial_regex,
            rank_partial_regex,
            rank_partial_regex,
            rank_partial_regex,
            rank_partial_regex,
            rank_partial_regex,
            color_partial_regex,
            castling_partial_regex,
            en_passant_partial_regex,
            halfmove_partial_regex,
            fullmove_partial_regex
        );
        let re = Regex::new(&full_fen_regex).unwrap();

        let caps = match re.captures(&fen) {
            Some(captures) => captures,
            None => return Err(format!("invalid FEN; could not parse board from `{}`", fen)),
        };

        // blank board
        let mut board = Board::new();

        // parse ranks
        for capture_group in 1..=8 {
            let rank = &caps[capture_group];
            let row = 8 - capture_group;
            let mut col = 0;

            for fen_char in rank.chars() {
                let coord = Coordinate::from_row_col(row, col);
                assert!(col < 8);
                match Piece::from_fen(fen_char) {
                    Some(piece) => {
                        board.put(coord, Some(piece));
                        col += 1;
                    }
                    None => {
                        // must be empty square. parse it and advance col counter
                        let empty_square_count = fen_char.to_digit(10).unwrap();
                        col += empty_square_count as usize;
                    }
                };
            }
        }

        Ok(board)
    }

    pub fn to_ascii(&self) -> String {
        let divider = "+---+---+---+---+---+---+---+---+\n";
        let rows: Vec<String> = (0..=7)
            .rev()
            .map(|row| {
                let cells: Vec<String> = (0..=7)
                    .map(|col| {
                        let coord = Coordinate::from_row_col(row, col);
                        match self.get(coord) {
                            Some(piece) => piece.to_fen(),
                            None => ' ',
                        }
                    })
                    .map(|ch| ch.to_string())
                    .collect();
                format!("| {} |\n", cells.join(" | "))
            })
            .collect();
        format!("{}{}{}", divider, rows.join(divider), divider)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use piece::Color;

    #[test]
    fn parse_fen() {
        // based off of examples from https://www.chess.com/terms/fen-chess
        let board = Board::from_fen("8/8/8/4p1K1/2k1P3/8/8/8 b - - 0 1").unwrap();
        println!("Testing board:\n{}", board.to_ascii());
        let tests = vec![
            (Coordinate::C4, Piece::King(Color::Black)),
            (Coordinate::E5, Piece::Pawn(Color::White)),
            (Coordinate::E4, Piece::Pawn(Color::Black)),
            (Coordinate::G5, Piece::King(Color::White)),
        ];

        for (coord, piece) in &tests {
            let _expected = Some(piece);
            assert!(matches!(board.get(*coord), _expected));
        }
        let occupied_squares: Vec<Coordinate> = tests
            .into_iter()
            .map(|(coord, _expected_piece)| coord.clone())
            .collect();

        for coord in Coordinate::all() {
            if occupied_squares.contains(&coord) {
                continue;
            }
            assert!(matches!(board.get(coord), None));
        }
    }
}
