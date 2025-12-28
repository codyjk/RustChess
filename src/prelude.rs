//! Common types re-exported for convenience.

pub use crate::board::{Board, Color, Piece};
pub use crate::chess_move::{
    CastleChessMove, ChessMove, EnPassantChessMove, PawnPromotionChessMove, StandardChessMove,
};
pub use common::bitboard::Bitboard;

