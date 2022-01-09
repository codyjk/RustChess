use super::color::Color;
use super::piece::Piece;
use super::square;
use super::{
    Board, BLACK_KINGSIDE_RIGHTS, BLACK_QUEENSIDE_RIGHTS, WHITE_KINGSIDE_RIGHTS,
    WHITE_QUEENSIDE_RIGHTS,
};
use regex::Regex;

pub const STARTING_POSITION_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

impl Board {
    pub fn from_fen(fen: &str) -> Result<Self, String> {
        let re = Regex::new(
            r"(?x)
            # `(?x)` - insignificant whitespace mode. makes it easier to comment
            # `\x20` - character code for a single space ` `
            ^
            ([pnbrqkPNBRQK1-8]{1,8}) # first rank
            /
            ([pnbrqkPNBRQK1-8]{1,8}) # second rank
            /
            ([pnbrqkPNBRQK1-8]{1,8}) # third rank
            /
            ([pnbrqkPNBRQK1-8]{1,8}) # fourth rank
            /
            ([pnbrqkPNBRQK1-8]{1,8}) # fifth rank
            /
            ([pnbrqkPNBRQK1-8]{1,8}) # sixth rank
            /
            ([pnbrqkPNBRQK1-8]{1,8}) # seventh rank
            /
            ([pnbrqkPNBRQK1-8]{1,8}) # eighth rank
            \x20
            (b|w)                    # current turn
            \x20
            ([kqKQ]{1,4}|-)          # castling rights
            \x20
            ([a-h][1-8]|-)           # en passant target square
            \x20
            (0|[1-9][0-9]*)          # halfmove count
            \x20
            ([1-9][0-9]*)            # fullmove count
            $

        ",
        )
        .unwrap();

        let caps = match re.captures(&fen) {
            Some(captures) => captures,
            None => return Err(format!("invalid FEN; could not parse board from `{}`", fen)),
        };

        // blank board
        let mut board = Self::new();

        // parse ranks
        for rank_capture_index in 1..=8 {
            let rank_str = &caps[rank_capture_index];
            let rank = (8 - rank_capture_index) as u8;
            let mut file = 0;

            for fen_char in rank_str.chars() {
                let square = square::from_rank_file(rank, file);
                assert!(file < 8);
                match Piece::from_fen(fen_char) {
                    Some((piece, color)) => {
                        board.put(square, piece, color).unwrap();
                        file += 1;
                    }
                    None => {
                        // must be empty square. parse it and advance col counter
                        let empty_square_count = fen_char.to_digit(10).unwrap() as u8;
                        file += empty_square_count;
                    }
                };
            }
        }

        // parse turn
        board.turn = match &caps[9] {
            "b" => Some(Color::Black),
            "w" => Some(Color::White),
            _ => None,
        }
        .unwrap();

        // parse castling rights
        let raw_rights = &caps[10];
        let mut lost_rights = 0b000;

        if !raw_rights.contains('K') {
            lost_rights |= WHITE_KINGSIDE_RIGHTS;
        }

        if !raw_rights.contains('Q') {
            lost_rights |= WHITE_QUEENSIDE_RIGHTS;
        }

        if !raw_rights.contains('k') {
            lost_rights |= BLACK_KINGSIDE_RIGHTS;
        }

        if !raw_rights.contains('q') {
            lost_rights |= BLACK_QUEENSIDE_RIGHTS;
        }

        board.lose_castle_rights(lost_rights);

        // parse en passant target square
        let en_passant_target = &caps[11];

        if !en_passant_target.contains('-') {
            let square = square::from_algebraic(en_passant_target);
            board.push_en_passant_target(square);
        }

        // halfmove clock
        let raw_halfmove_clock = &caps[12];
        let halfmove_clock = raw_halfmove_clock.parse::<u8>().unwrap();
        board.push_halfmove_clock(halfmove_clock);

        // fullmove clock
        let raw_fullmove_clock = &caps[13];
        let fullmove_clock = raw_fullmove_clock.parse::<u8>().unwrap();
        board.set_fullmove_clock(fullmove_clock);

        Ok(board)
    }

    pub fn to_fen(&self) -> String {
        let mut fen_rows = vec![];
        for rank in (0..8).rev() {
            let mut pieces = vec![];
            let mut empty_square_count = 0;
            for file in 0..8 {
                let sq = square::from_rank_file(rank, file);
                if let Some((piece, color)) = self.get(sq) {
                    if empty_square_count > 0 {
                        pieces.push(empty_square_count.to_string());
                    }
                    empty_square_count = 0;
                    pieces.push(piece.to_fen(color).to_string());
                } else {
                    empty_square_count += 1;
                }
            }
            if empty_square_count > 0 {
                pieces.push(empty_square_count.to_string());
            }
            fen_rows.push(pieces.join(""));
        }

        let fen_turn = match self.turn() {
            Color::Black => 'b',
            Color::White => 'w',
        };

        let castle_rights = self.peek_castle_rights();
        let mut fen_castle_rights = vec![];
        if castle_rights == 0b0000 {
            fen_castle_rights.push("-");
        } else {
            if castle_rights & WHITE_KINGSIDE_RIGHTS > 0 {
                fen_castle_rights.push("K");
            }

            if castle_rights & WHITE_QUEENSIDE_RIGHTS > 0 {
                fen_castle_rights.push("Q");
            }

            if castle_rights & BLACK_KINGSIDE_RIGHTS > 0 {
                fen_castle_rights.push("k");
            }

            if castle_rights & BLACK_QUEENSIDE_RIGHTS > 0 {
                fen_castle_rights.push("q");
            }
        }

        let fen_en_passant = match self.peek_en_passant_target() {
            0 => "-".to_string(),
            sq => square::to_algebraic(sq).to_lowercase(),
        };

        let fen = format!(
            "{} {} {} {} {} {}",
            fen_rows.join("/"),
            fen_turn,
            fen_castle_rights.join(""),
            fen_en_passant,
            self.halfmove_clock(),
            self.fullmove_clock(),
        );
        fen
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
        let board = Board::from_fen("8/8/8/4p1K1/2k1P3/8/8/8 b - - 4 11").unwrap();
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

        assert_eq!(Color::Black, board.turn());
        assert_eq!(0b0000, board.peek_castle_rights());
        assert_eq!(0, board.peek_en_passant_target());
        assert_eq!(4, board.halfmove_clock());
        assert_eq!(11, board.fullmove_clock());
    }

    #[test]
    fn test_to_fen() {
        let test_fen = "8/8/8/4p1K1/2k1P3/8/8/8 b - - 4 11";
        let board = Board::from_fen(test_fen).unwrap();
        assert_eq!(test_fen, board.to_fen());
        assert_eq!(STARTING_POSITION_FEN, Board::starting_position().to_fen());
    }
}
