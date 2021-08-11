use super::color::Color;
use super::piece::Piece;
use super::square;
use super::Board;
use regex::Regex;

pub const STARTING_POSITION_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

impl Board {
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
    pub fn from_fen(fen: &str) -> Result<Self, String> {
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
        let mut board = Self::new();

        // parse ranks
        for capture_group in 1..=8 {
            let rank = &caps[capture_group];
            let row = 8 - capture_group;
            let mut col = 0;

            for fen_char in rank.chars() {
                let square = square::from_row_col(row, col);
                assert!(col < 8);
                match Piece::from_fen(fen_char) {
                    Some((piece, color)) => {
                        board.put(square, piece, color).unwrap();
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
}

impl Piece {
    pub fn to_fen(&self, color: Color) -> char {
        match (self, color) {
            (Piece::Bishop, Color::Black) => 'b',
            (Piece::Bishop, Color::White) => 'B',
            (Piece::King, Color::Black) => 'k',
            (Piece::King, Color::White) => 'K',
            (Piece::Knight, Color::Black) => 'n',
            (Piece::Knight, Color::White) => 'N',
            (Piece::Pawn, Color::Black) => 'p',
            (Piece::Pawn, Color::White) => 'P',
            (Piece::Queen, Color::Black) => 'q',
            (Piece::Queen, Color::White) => 'Q',
            (Piece::Rook, Color::Black) => 'r',
            (Piece::Rook, Color::White) => 'R',
        }
    }

    pub fn from_fen(c: char) -> Option<(Piece, Color)> {
        match c {
            'b' => Some((Piece::Bishop, Color::Black)),
            'B' => Some((Piece::Bishop, Color::White)),
            'k' => Some((Piece::King, Color::Black)),
            'K' => Some((Piece::King, Color::White)),
            'n' => Some((Piece::Knight, Color::Black)),
            'N' => Some((Piece::Knight, Color::White)),
            'p' => Some((Piece::Pawn, Color::Black)),
            'P' => Some((Piece::Pawn, Color::White)),
            'q' => Some((Piece::Queen, Color::Black)),
            'Q' => Some((Piece::Queen, Color::White)),
            'r' => Some((Piece::Rook, Color::Black)),
            'R' => Some((Piece::Rook, Color::White)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_fen() {
        // based off of examples from https://www.chess.com/terms/fen-chess
        let board = Board::from_fen("8/8/8/4p1K1/2k1P3/8/8/8 b - - 0 1").unwrap();
        println!("Testing board:\n{}", board.to_ascii());
        let tests = vec![
            (square::C4, Piece::King, Color::Black),
            (square::E5, Piece::Pawn, Color::Black),
            (square::E4, Piece::Pawn, Color::White),
            (square::G5, Piece::King, Color::White),
        ];

        for (square, piece, color) in &tests {
            assert_eq!(board.get(*square).unwrap(), (*piece, *color));
        }
        let occupied_squares: Vec<u64> = tests
            .into_iter()
            .map(|(square, _expected_piece, _expected_color)| square.clone())
            .collect();

        for square in &square::ORDERED {
            if occupied_squares.contains(&square) {
                continue;
            }
            assert!(matches!(board.get(*square), None));
        }
    }
}
