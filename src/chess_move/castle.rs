use core::fmt;

use super::Capture;
use crate::board::{
    castle_rights_bitmask::{
        BLACK_KINGSIDE_RIGHTS, BLACK_QUEENSIDE_RIGHTS, WHITE_KINGSIDE_RIGHTS,
        WHITE_QUEENSIDE_RIGHTS,
    },
    color::Color,
    error::BoardError,
    piece::Piece,
    Board,
};
use common::bitboard::bitboard::Bitboard;
use common::bitboard::square::*;

/// Represents a castle move in chess. This struct encapsulates the logic for applying
/// and undoing a castle move on a chess board.
/// The intended entry points for this struct are the `castle_kingside` and `castle_queenside`.
/// As such, the struct is not intended to be constructed directly.
#[derive(PartialEq, Clone, Eq, PartialOrd, Ord)]
pub struct CastleChessMove {
    /// The square the king is moving from
    from_square: Bitboard,

    /// The square the king is moving to
    to_square: Bitboard,
}

impl CastleChessMove {
    fn new(from_square: Bitboard, to_square: Bitboard) -> Self {
        Self {
            from_square,
            to_square,
        }
    }

    pub fn castle_kingside(color: Color) -> Self {
        match color {
            Color::White => Self::new(E1, G1),
            Color::Black => Self::new(E8, G8),
        }
    }

    pub fn castle_queenside(color: Color) -> Self {
        match color {
            Color::White => Self::new(E1, C1),
            Color::Black => Self::new(E8, C8),
        }
    }

    pub fn to_square(&self) -> Bitboard {
        self.to_square
    }

    pub fn from_square(&self) -> Bitboard {
        self.from_square
    }

    pub fn apply(&self, board: &mut Board) -> Result<Option<Capture>, BoardError> {
        let CastleChessMove {
            from_square: king_from,
            to_square: king_to,
        } = self;

        let kingside = match *king_to {
            b if b == *king_from << 2 => true,
            b if b == *king_from >> 2 => false,
            _ => return Err(BoardError::InvalidCastleMoveError),
        };

        let overlaps_first_rank = king_from.overlaps(Bitboard::RANK_1);
        let overlaps_eighth_rank = king_from.overlaps(Bitboard::RANK_8);
        let color = match (overlaps_first_rank, overlaps_eighth_rank) {
            (true, false) => Color::White,
            (false, true) => Color::Black,
            _ => return Err(BoardError::InvalidCastleMoveError),
        };

        let (rook_from, rook_to) = match (color, kingside) {
            (Color::White, true) => (H1, F1),
            (Color::White, false) => (A1, D1),
            (Color::Black, true) => (H8, F8),
            (Color::Black, false) => (A8, D8),
        };

        if board.get(*king_from) != Some((Piece::King, color)) {
            return Err(BoardError::InvalidCastleStateError {
                msg: "king_from is not a king",
            });
        }

        if board.get(*king_to).is_some() {
            return Err(BoardError::InvalidCastleStateError {
                msg: "king_to is not empty",
            });
        }

        if board.get(rook_from) != Some((Piece::Rook, color)) {
            return Err(BoardError::InvalidCastleStateError {
                msg: "rook_from is not a rook",
            });
        }

        if board.get(rook_to).is_some() {
            return Err(BoardError::InvalidCastleStateError {
                msg: "rook_to is not empty",
            });
        }

        board.remove(*king_from).unwrap();
        board.put(*king_to, Piece::King, color).unwrap();
        board.remove(rook_from).unwrap();
        board.put(rook_to, Piece::Rook, color).unwrap();

        let lost_castle_rights = match color {
            Color::White => WHITE_KINGSIDE_RIGHTS | WHITE_QUEENSIDE_RIGHTS,
            Color::Black => BLACK_KINGSIDE_RIGHTS | BLACK_QUEENSIDE_RIGHTS,
        };

        board.increment_halfmove_clock();
        board.increment_fullmove_clock();
        board.push_en_passant_target(Bitboard::EMPTY);
        board.lose_castle_rights(lost_castle_rights);

        Ok(None)
    }

    pub fn undo(&self, board: &mut Board) -> Result<Option<Capture>, BoardError> {
        let CastleChessMove {
            from_square: king_from,
            to_square: king_to,
        } = self;

        let kingside = match *king_to {
            b if b == *king_from << 2 => true,
            b if b == *king_from >> 2 => false,
            _ => return Err(BoardError::InvalidCastleMoveError),
        };

        let overlaps_first_rank = king_from.overlaps(Bitboard::RANK_1);
        let overlaps_eighth_rank = king_from.overlaps(Bitboard::RANK_8);
        let color = match (overlaps_first_rank, overlaps_eighth_rank) {
            (true, false) => Color::White,
            (false, true) => Color::Black,
            _ => return Err(BoardError::InvalidCastleMoveError),
        };

        let (rook_from, rook_to) = match (color, kingside) {
            (Color::White, true) => (H1, F1),
            (Color::White, false) => (A1, D1),
            (Color::Black, true) => (H8, F8),
            (Color::Black, false) => (A8, D8),
        };

        if board.get(*king_to) != Some((Piece::King, color)) {
            return Err(BoardError::InvalidCastleStateError {
                msg: "king_to is not a king",
            });
        }

        if board.get(*king_from).is_some() {
            return Err(BoardError::InvalidCastleStateError {
                msg: "king_from is not empty",
            });
        }

        if board.get(rook_to) != Some((Piece::Rook, color)) {
            return Err(BoardError::InvalidCastleStateError {
                msg: "rook_to is not a rook",
            });
        }

        if board.get(rook_from).is_some() {
            return Err(BoardError::InvalidCastleStateError {
                msg: "rook_from is not empty",
            });
        }

        board.remove(*king_to).unwrap();
        board.put(*king_from, Piece::King, color).unwrap();
        board.remove(rook_to).unwrap();
        board.put(rook_from, Piece::Rook, color).unwrap();

        // Revert the board state.
        board.decrement_fullmove_clock();
        board.pop_halfmove_clock();
        board.pop_en_passant_target();
        board.pop_castle_rights();

        Ok(None)
    }
}

impl fmt::Display for CastleChessMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "castle {} {}",
            to_algebraic(self.from_square).to_lowercase(),
            to_algebraic(self.to_square).to_lowercase(),
        )
    }
}

impl fmt::Debug for CastleChessMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format!("{}", self).fmt(f)
    }
}

#[macro_export]
macro_rules! castle_kingside {
    ($color:expr) => {
        ChessMove::Castle(CastleChessMove::castle_kingside($color))
    };
}

#[macro_export]
macro_rules! castle_queenside {
    ($color:expr) => {
        ChessMove::Castle(CastleChessMove::castle_queenside($color))
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{chess_move::ChessMove, chess_position};

    #[test]
    fn test_apply_and_undo_castle_white_kingside() {
        let mut board = chess_position! {
            ........
            ........
            ........
            ........
            ........
            ........
            ........
            ....K..R
        };
        println!("Testing board:\n{}", board);

        let castle = castle_kingside!(Color::White);

        castle.apply(&mut board).unwrap();
        println!("After applying castle:\n{}", board);
        assert_eq!(Some((Piece::King, Color::White)), board.get(G1));
        assert_eq!(Some((Piece::Rook, Color::White)), board.get(F1));

        castle.undo(&mut board).unwrap();
        println!("After undoing castle:\n{}", board);
        assert_eq!(Some((Piece::King, Color::White)), board.get(E1));
        assert_eq!(Some((Piece::Rook, Color::White)), board.get(H1));
    }

    #[test]
    fn test_apply_and_undo_castle_black_kingside() {
        let mut board = chess_position! {
            ....k..r
            ........
            ........
            ........
            ........
            ........
            ........
            ........
        };
        println!("Testing board:\n{}", board);

        let castle = castle_kingside!(Color::Black);

        castle.apply(&mut board).unwrap();
        println!("After applying castle:\n{}", board);
        assert_eq!(Some((Piece::King, Color::Black)), board.get(G8));
        assert_eq!(Some((Piece::Rook, Color::Black)), board.get(F8));

        castle.undo(&mut board).unwrap();
        println!("After undoing castle:\n{}", board);
        assert_eq!(Some((Piece::King, Color::Black)), board.get(E8));
        assert_eq!(Some((Piece::Rook, Color::Black)), board.get(H8));
    }

    #[test]
    fn test_apply_and_undo_castle_white_queenside() {
        let mut board = chess_position! {
            ........
            ........
            ........
            ........
            ........
            ........
            ........
            R...K...
        };
        println!("Testing board:\n{}", board);

        let castle = castle_queenside!(Color::White);

        castle.apply(&mut board).unwrap();
        println!("After applying castle:\n{}", board);
        assert_eq!(Some((Piece::King, Color::White)), board.get(C1));
        assert_eq!(Some((Piece::Rook, Color::White)), board.get(D1));

        castle.undo(&mut board).unwrap();
        println!("After undoing castle:\n{}", board);
        assert_eq!(Some((Piece::King, Color::White)), board.get(E1));
        assert_eq!(Some((Piece::Rook, Color::White)), board.get(A1));
    }

    #[test]
    fn test_apply_and_undo_castle_black_queenside() {
        let mut board = chess_position! {
            r...k...
            ........
            ........
            ........
            ........
            ........
            ........
            ........
        };
        println!("Testing board:\n{}", board);

        let castle = castle_queenside!(Color::Black);

        castle.apply(&mut board).unwrap();
        println!("After applying castle:\n{}", board);
        assert_eq!(Some((Piece::King, Color::Black)), board.get(C8));
        assert_eq!(Some((Piece::Rook, Color::Black)), board.get(D8));

        castle.undo(&mut board).unwrap();
        println!("After undoing castle:\n{}", board);
        assert_eq!(Some((Piece::King, Color::Black)), board.get(E8));
        assert_eq!(Some((Piece::Rook, Color::Black)), board.get(A8));
    }

    #[test]
    fn test_zobrist_hashing_reversible_for_castle() {
        let mut board = chess_position! {
            r...k..r
            ........
            ........
            ........
            ........
            ........
            ........
            ........
        };
        let initial_hash = board.current_position_hash();

        let castle = castle_queenside!(Color::Black);

        println!("applying castle queenside");
        castle.apply(&mut board).unwrap();
        assert_ne!(
            initial_hash,
            board.current_position_hash(),
            "hash should change after applying queenside castle"
        );

        println!("undoing castle queenside");
        castle.undo(&mut board).unwrap();
        assert_eq!(
            initial_hash,
            board.current_position_hash(),
            "hash should be equal after undoing queenside castle"
        );

        let castle = castle_kingside!(Color::Black);

        println!("applying castle kingside");
        castle.apply(&mut board).unwrap();
        assert_ne!(
            initial_hash,
            board.current_position_hash(),
            "hash should change after applying kingside castle"
        );

        println!("undoing castle kingside");
        castle.undo(&mut board).unwrap();
        assert_eq!(
            initial_hash,
            board.current_position_hash(),
            "hash should be equal after undoing kingside castle"
        );
    }
}
