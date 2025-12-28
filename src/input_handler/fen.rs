use crate::board::{
    castle_rights_bitmask::*, color::Color, error::BoardError, piece::Piece, Board,
};
use common::bitboard::Square;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FenParseError {
    #[error("Wrong number of fields")]
    WrongNumberOfFields,
    #[error("Invalid piece character: {invalid_character:?}")]
    InvalidPieceCharacter { invalid_character: char },
    #[error("Wrong number of ranks: 8 expected, {rank_count:?} given")]
    InvalidRankCount { rank_count: usize },
    #[error("Rank too long: {invalid_rank:?}")]
    InvalidRankLength { invalid_rank: String },
    #[error("Error placing piece: {board_error:?}")]
    ErrorPlacingPiece { board_error: BoardError },
    #[error("Rank incomplete: {incomplete_rank:?}")]
    IncompleteRank { incomplete_rank: String },
    #[error("Invalid color: {invalid_color:?}")]
    InvalidColor { invalid_color: String },
    #[error("Invalid castling rights: {invalid_castling:?}")]
    InvalidCastlingRights { invalid_castling: char },
    #[error("Invalid en passant {component:?}: {value:?}")]
    InvalidEnPassant { component: String, value: String },
    #[error("Invalid halfmove clock: {invalid_clock:?}")]
    InvalidHalfmoveClock { invalid_clock: String },
    #[error("Invalid fullmove number: {invalid_number:?}")]
    InvalidFullmoveNumber { invalid_number: String },
}

type FenResult<T> = Result<T, FenParseError>;

pub const STARTING_POSITION_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

/// Parses a FEN (Forsyth–Edwards Notation) string into a Board.
/// FEN string contains 6 fields: piece placement, active color, castling rights,
/// en passant target square, halfmove clock, and fullmove number.
pub fn parse_fen(fen: &str) -> FenResult<Board> {
    let fields = split_fen_fields(fen)?;
    let mut board = Board::new();

    parse_piece_placement(&mut board, fields.position)?;
    parse_active_color(&mut board, fields.active_color)?;
    parse_castle_rights(&mut board, fields.castle_rights)?;
    parse_en_passant(&mut board, fields.en_passant)?;
    parse_halfmove_clock(&mut board, fields.halfmove_clock)?;
    parse_fullmove_number(&mut board, fields.fullmove_number)?;

    Ok(board)
}

/// Represents the six fields in a FEN string
struct FenFields<'a> {
    position: &'a str,
    active_color: &'a str,
    castle_rights: &'a str,
    en_passant: &'a str,
    halfmove_clock: &'a str,
    fullmove_number: &'a str,
}

/// Splits a FEN string into its six component fields
fn split_fen_fields(fen: &str) -> FenResult<FenFields> {
    let parts: Vec<&str> = fen.split_whitespace().collect();
    if parts.len() != 6 {
        return Err(FenParseError::WrongNumberOfFields);
    }

    Ok(FenFields {
        position: parts[0],
        active_color: parts[1],
        castle_rights: parts[2],
        en_passant: parts[3],
        halfmove_clock: parts[4],
        fullmove_number: parts[5],
    })
}

/// Maps a FEN piece character to its corresponding piece type and color
fn parse_piece_char(c: char) -> FenResult<(Piece, Color)> {
    match c {
        'P' => Ok((Piece::Pawn, Color::White)),
        'N' => Ok((Piece::Knight, Color::White)),
        'B' => Ok((Piece::Bishop, Color::White)),
        'R' => Ok((Piece::Rook, Color::White)),
        'Q' => Ok((Piece::Queen, Color::White)),
        'K' => Ok((Piece::King, Color::White)),
        'p' => Ok((Piece::Pawn, Color::Black)),
        'n' => Ok((Piece::Knight, Color::Black)),
        'b' => Ok((Piece::Bishop, Color::Black)),
        'r' => Ok((Piece::Rook, Color::Black)),
        'q' => Ok((Piece::Queen, Color::Black)),
        'k' => Ok((Piece::King, Color::Black)),
        _ => Err(FenParseError::InvalidPieceCharacter {
            invalid_character: c,
        }),
    }
}

/// Parses the piece placement section of the FEN string
fn parse_piece_placement(board: &mut Board, position: &str) -> FenResult<()> {
    let ranks: Vec<&str> = position.split('/').collect();
    if ranks.len() != 8 {
        return Err(FenParseError::InvalidRankCount {
            rank_count: ranks.len(),
        });
    }

    for (rank_idx, rank) in ranks.iter().enumerate() {
        parse_rank(board, rank, 7 - rank_idx as u8)?;
    }

    Ok(())
}

/// Parses a single rank of the piece placement section
fn parse_rank(board: &mut Board, rank: &str, rank_number: u8) -> FenResult<()> {
    let mut file = 0u8;

    for c in rank.chars() {
        if file >= 8 {
            return Err(FenParseError::InvalidRankLength {
                invalid_rank: rank.to_string(),
            });
        }

        if let Some(empty_squares) = c.to_digit(10) {
            file += empty_squares as u8;
        } else {
            let (piece, color) = parse_piece_char(c)?;
            board
                .put(Square::from_rank_file(rank_number, file), piece, color)
                .map_err(|e| FenParseError::ErrorPlacingPiece { board_error: e })?;
            file += 1;
        }
    }

    if file != 8 {
        return Err(FenParseError::IncompleteRank {
            incomplete_rank: rank.to_string(),
        });
    }

    Ok(())
}

/// Parses the active color field
fn parse_active_color(board: &mut Board, active_color: &str) -> FenResult<()> {
    match active_color {
        "w" => {
            board.set_turn(Color::White);
            Ok(())
        }
        "b" => {
            board.set_turn(Color::Black);
            Ok(())
        }
        _ => Err(FenParseError::InvalidColor {
            invalid_color: active_color.to_string(),
        }),
    }
}

/// Parses the castling rights field
fn parse_castle_rights(board: &mut Board, castle_rights: &str) -> FenResult<()> {
    if castle_rights == "-" {
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);
        return Ok(());
    }

    let mut rights = 0u8;
    for c in castle_rights.chars() {
        rights |= match c {
            'K' => WHITE_KINGSIDE_RIGHTS,
            'Q' => WHITE_QUEENSIDE_RIGHTS,
            'k' => BLACK_KINGSIDE_RIGHTS,
            'q' => BLACK_QUEENSIDE_RIGHTS,
            _ => {
                return Err(FenParseError::InvalidCastlingRights {
                    invalid_castling: c,
                })
            }
        };
    }
    board.lose_castle_rights(!rights);
    Ok(())
}

/// Parses the en passant target square field
fn parse_en_passant(board: &mut Board, en_passant: &str) -> FenResult<()> {
    if en_passant == "-" {
        return Ok(());
    }

    if en_passant.len() != 2 {
        return Err(FenParseError::InvalidEnPassant {
            component: "square".to_string(),
            value: en_passant.to_string(),
        });
    }

    let file = en_passant
        .chars()
        .nth(0)
        .ok_or_else(|| FenParseError::InvalidEnPassant {
            component: "file".to_string(),
            value: en_passant.to_string(),
        })?;
    let rank = en_passant
        .chars()
        .nth(1)
        .ok_or_else(|| FenParseError::InvalidEnPassant {
            component: "rank".to_string(),
            value: en_passant.to_string(),
        })?;

    if !file.is_ascii_lowercase()
        || file < 'a'
        || file > 'h'
        || !rank.is_ascii_digit()
        || rank < '1'
        || rank > '8'
    {
        return Err(FenParseError::InvalidEnPassant {
            component: "square".to_string(),
            value: en_passant.to_string(),
        });
    }

    let file = file as u8 - b'a';
    let rank = rank as u8 - b'1';
    board.push_en_passant_target(Some(Square::from_rank_file(rank, file)));
    Ok(())
}

/// Parses the halfmove clock field
fn parse_halfmove_clock(board: &mut Board, halfmove_clock: &str) -> FenResult<()> {
    let halfmove =
        halfmove_clock
            .parse::<u8>()
            .map_err(|_| FenParseError::InvalidHalfmoveClock {
                invalid_clock: halfmove_clock.to_string(),
            })?;
    board.push_halfmove_clock(halfmove);
    Ok(())
}

/// Parses the fullmove number field
fn parse_fullmove_number(board: &mut Board, fullmove_number: &str) -> FenResult<()> {
    let fullmove =
        fullmove_number
            .parse::<u8>()
            .map_err(|_| FenParseError::InvalidFullmoveNumber {
                invalid_number: fullmove_number.to_string(),
            })?;
    board.set_fullmove_clock(fullmove);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::bitboard::bitboard::Bitboard;

    #[test]
    fn test_parse_starting_position() {
        let board: Board = STARTING_POSITION_FEN.parse().unwrap();
        assert_eq!(
            board.current_position_hash(),
            Board::default().current_position_hash()
        );
    }

    #[test]
    fn test_parse_complex_position() {
        let fen = "r1bqk2r/ppp2ppp/2n2n2/2bpp3/4P3/2PP1N2/PP1N1PPP/R1BQKB1R b KQkq - 0 6";
        let board = parse_fen(fen).unwrap();

        // Verify some key aspects of the position
        assert_eq!(board.turn(), Color::Black);
        assert_eq!(board.halfmove_clock(), 0);
        assert_eq!(board.fullmove_clock(), 6);

        // Verify a few piece positions
        assert_eq!(
            board.get(Square::from_rank_file(7, 0)),
            Some((Piece::Rook, Color::Black))
        );
        assert_eq!(
            board.get(Square::from_rank_file(4, 4)),
            Some((Piece::Pawn, Color::Black))
        );
    }

    #[test]
    fn test_invalid_fen() {
        // Test invalid number of fields
        assert!(parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -").is_err());

        // Test invalid piece placement
        assert!(parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBN w KQkq - 0 1").is_err());

        // Test invalid active color
        assert!(parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR x KQkq - 0 1").is_err());

        // Test invalid castling rights
        assert!(parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w XYZx - 0 1").is_err());
    }

    #[test]
    fn test_empty_squares() {
        let fen = "8/8/8/8/8/8/8/8 w - - 0 1";
        let board = parse_fen(fen).unwrap();
        assert_eq!(board.occupied(), Bitboard::EMPTY);
    }

    #[test]
    fn test_en_passant_parsing() {
        let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
        let board = parse_fen(fen).unwrap();
        assert_eq!(board.peek_en_passant_target(), Some(Square::from_rank_file(2, 4)));
    }

    #[test]
    fn test_castle_rights() {
        // Test all castle rights
        let fen = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
        let board = parse_fen(fen).unwrap();
        assert_eq!(board.peek_castle_rights(), ALL_CASTLE_RIGHTS);

        // Test no castle rights
        let fen = "r3k2r/8/8/8/8/8/8/R3K2R w - - 0 1";
        let board = parse_fen(fen).unwrap();
        assert_eq!(board.peek_castle_rights(), 0);

        // Test partial castle rights
        let fen = "r3k2r/8/8/8/8/8/8/R3K2R w Kq - 0 1";
        let board = parse_fen(fen).unwrap();
        assert_eq!(
            board.peek_castle_rights(),
            WHITE_KINGSIDE_RIGHTS | BLACK_QUEENSIDE_RIGHTS
        );
    }
}
