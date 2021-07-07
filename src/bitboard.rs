pub mod piece;
pub mod square;

use piece::{Color, Piece};
use regex::Regex;
use square::Square;

const EMPTY_BOARD: u64 = 0;
const STARTING_POSITION_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

pub struct Bitboard {
    white: Pieces,
    black: Pieces,

    // derived
    occupied: u64,
}

impl Pieces {
    fn new() -> Self {
        Pieces {
            bishops: EMPTY_BOARD,
            kings: EMPTY_BOARD,
            knights: EMPTY_BOARD,
            pawns: EMPTY_BOARD,
            queens: EMPTY_BOARD,
            rooks: EMPTY_BOARD,

            occupied: EMPTY_BOARD,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Pieces {
    pawns: u64,
    rooks: u64,
    knights: u64,
    bishops: u64,
    kings: u64,
    queens: u64,
    occupied: u64,
}

#[derive(Clone, Copy, PartialEq)]
pub struct ChessMove {
    pub from_square: Square,
    pub to_square: Square,
}

impl ChessMove {
    pub fn from_algebraic(algebraic_move: String) -> Result<Self, &'static str> {
        let re = Regex::new("^([a-h][1-8])([a-h][1-8])$").unwrap();
        let caps = match re.captures(&algebraic_move) {
            Some(captures) => captures,
            None => return Err("invalid move"),
        };

        Ok(Self {
            from_square: Square::from_algebraic(&caps[1]),
            to_square: Square::from_algebraic(&caps[2]),
        })
    }
}

impl Bitboard {
    pub fn new() -> Self {
        Bitboard {
            white: Pieces::new(),
            black: Pieces::new(),
            occupied: EMPTY_BOARD,
        }
    }

    pub fn starting_position() -> Self {
        Self::from_fen(STARTING_POSITION_FEN).unwrap()
    }

    pub fn get(&self, square: Square) -> Option<(Piece, Color)> {
        let square_bit = square.to_bit();
        let color = if square_bit & self.white.occupied > 0 {
            Color::White
        } else if square_bit & self.black.occupied > 0 {
            Color::Black
        } else {
            return None;
        };

        let piece = self.get_piece_for_color(square, color).unwrap();

        Some((piece, color))
    }

    pub fn is_occupied(&self, square: Square) -> bool {
        self.get(square).is_some()
    }

    fn get_piece_for_color(&self, square: Square, color: Color) -> Option<Piece> {
        let square_bit = square.to_bit();
        let pieces = match color {
            Color::White => self.white,
            Color::Black => self.black,
        };

        if square_bit & pieces.bishops > 0 {
            return Some(Piece::Bishop);
        } else if square_bit & pieces.kings > 0 {
            return Some(Piece::King);
        } else if square_bit & pieces.knights > 0 {
            return Some(Piece::Knight);
        } else if square_bit & pieces.pawns > 0 {
            return Some(Piece::Pawn);
        } else if square_bit & pieces.queens > 0 {
            return Some(Piece::Queen);
        } else if square_bit & pieces.rooks > 0 {
            return Some(Piece::Rook);
        }

        None
    }

    /// Puts a `piece` on the requested `square` if it is empty
    pub fn put(&mut self, square: Square, piece: Piece, color: Color) -> Result<(), &'static str> {
        if self.is_occupied(square) {
            return Err("that square already has a piece on it");
        }

        let square_bit = square.to_bit();

        let mut pieces = match color {
            Color::White => &mut self.white,
            Color::Black => &mut self.black,
        };

        match piece {
            Piece::Bishop => pieces.bishops |= square_bit,
            Piece::King => pieces.kings |= square_bit,
            Piece::Knight => pieces.knights |= square_bit,
            Piece::Pawn => pieces.pawns |= square_bit,
            Piece::Queen => pieces.queens |= square_bit,
            Piece::Rook => pieces.rooks |= square_bit,
        };

        pieces.occupied |= square_bit;
        self.occupied |= square_bit;

        Ok(())
    }

    pub fn remove(&mut self, square: Square) -> Option<(Piece, Color)> {
        let removed = self.get(square);
        let (removed_piece, removed_color) = match removed {
            Some((piece, color)) => (piece, color),
            None => return None,
        };

        let square_bit = square.to_bit();

        let mut pieces = match removed_color {
            Color::White => &mut self.white,
            Color::Black => &mut self.black,
        };

        match removed_piece {
            Piece::Bishop => pieces.bishops ^= square_bit,
            Piece::King => pieces.kings ^= square_bit,
            Piece::Knight => pieces.knights ^= square_bit,
            Piece::Pawn => pieces.pawns ^= square_bit,
            Piece::Queen => pieces.queens ^= square_bit,
            Piece::Rook => pieces.rooks ^= square_bit,
        };

        pieces.occupied ^= square_bit;
        self.occupied ^= square_bit;

        removed
    }

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
                let square = Square::from_row_col(row, col);
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

    pub fn to_ascii(&self) -> String {
        let divider = "+---+---+---+---+---+---+---+---+\n";
        let rows: Vec<String> = (0..=7)
            .rev()
            .map(|row| {
                let cells: Vec<String> = (0..=7)
                    .map(|col| {
                        let square = Square::from_row_col(row, col);
                        match self.get(square) {
                            Some((piece, color)) => piece.to_fen(color),
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

    #[test]
    fn test_parse_fen() {
        // based off of examples from https://www.chess.com/terms/fen-chess
        let board = Bitboard::from_fen("8/8/8/4p1K1/2k1P3/8/8/8 b - - 0 1").unwrap();
        println!("Testing board:\n{}", board.to_ascii());
        let tests = vec![
            (Square::C4, Piece::King, Color::Black),
            (Square::E5, Piece::Pawn, Color::Black),
            (Square::E4, Piece::Pawn, Color::White),
            (Square::G5, Piece::King, Color::White),
        ];

        for (square, piece, color) in &tests {
            assert_eq!(board.get(*square).unwrap(), (*piece, *color));
        }
        let occupied_squares: Vec<Square> = tests
            .into_iter()
            .map(|(square, _expected_piece, _expected_color)| square.clone())
            .collect();

        for square in Square::ordered() {
            if occupied_squares.contains(&square) {
                continue;
            }
            assert!(matches!(board.get(square), None));
        }
    }

    #[test]
    fn test_apply_chess_move() {
        let mut board = Bitboard::starting_position();
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
                })
                .unwrap();
            assert_eq!(board.get(*to_square).unwrap(), *moved);
            assert_eq!(captured, *expected_capture);
            println!("New board state:\n{}", board.to_ascii());
        }
    }
}
