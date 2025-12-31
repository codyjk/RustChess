//! FEN serialization - converts Board to FEN string.

use crate::board::Board;
use common::bitboard::Square;

/// Converts a Board to FEN (Forsyth–Edwards Notation) string.
pub fn to_fen(board: &Board) -> String {
    let mut fen = String::new();

    // 1. Piece placement
    for rank in (0..8).rev() {
        let mut empty_count = 0;
        for file in 0..8 {
            let square = Square::from_rank_file(rank, file);
            if let Some((piece, color)) = board.get(square) {
                if empty_count > 0 {
                    fen.push_str(&empty_count.to_string());
                    empty_count = 0;
                }
                fen.push(piece.to_fen_char(color));
            } else {
                empty_count += 1;
            }
        }
        if empty_count > 0 {
            fen.push_str(&empty_count.to_string());
        }
        if rank > 0 {
            fen.push('/');
        }
    }

    // 2. Active color
    fen.push(' ');
    fen.push(match board.turn() {
        crate::board::color::Color::White => 'w',
        crate::board::color::Color::Black => 'b',
    });

    // 3. Castling rights
    fen.push(' ');
    let castle_rights = board.peek_castle_rights();
    if castle_rights.is_empty() {
        fen.push('-');
    } else {
        use crate::board::castle_rights::CastleRights;
        if castle_rights.contains(CastleRights::white_kingside()) {
            fen.push('K');
        }
        if castle_rights.contains(CastleRights::white_queenside()) {
            fen.push('Q');
        }
        if castle_rights.contains(CastleRights::black_kingside()) {
            fen.push('k');
        }
        if castle_rights.contains(CastleRights::black_queenside()) {
            fen.push('q');
        }
    }

    // 4. En passant target square
    fen.push(' ');
    if let Some(ep_square) = board.peek_en_passant_target() {
        fen.push_str(ep_square.to_algebraic());
    } else {
        fen.push('-');
    }

    // 5. Halfmove clock
    fen.push(' ');
    fen.push_str(&board.halfmove_clock().value().to_string());

    // 6. Fullmove number
    fen.push(' ');
    fen.push_str(&board.fullmove_clock().value().to_string());

    fen
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_handler::fen::STARTING_POSITION_FEN;

    #[test]
    fn test_starting_position_fen() {
        let board = Board::default();
        let fen = to_fen(&board);
        assert_eq!(fen, STARTING_POSITION_FEN);
    }

    #[test]
    fn test_roundtrip() {
        let original_fen = "r1bqk2r/ppp2ppp/2n2n2/2bpp3/4P3/2PP1N2/PP1N1PPP/R1BQKB1R b KQkq - 0 6";
        let board: Board = original_fen.parse().unwrap();
        let serialized_fen = to_fen(&board);
        assert_eq!(serialized_fen, original_fen);
    }

    #[test]
    fn test_en_passant() {
        let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
        let board: Board = fen.parse().unwrap();
        let serialized = to_fen(&board);
        assert_eq!(serialized, fen);
    }

    #[test]
    fn test_no_castle_rights() {
        let fen = "r3k2r/8/8/8/8/8/8/R3K2R w - - 0 1";
        let board: Board = fen.parse().unwrap();
        let serialized = to_fen(&board);
        assert_eq!(serialized, fen);
    }
}
