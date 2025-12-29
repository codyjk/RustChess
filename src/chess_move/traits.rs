use crate::board::{error::BoardError, Board};
use common::bitboard::Square;

use super::chess_move_effect::ChessMoveEffect;

/// Trait defining the common interface for all chess move types.
///
/// All chess move variants (Standard, PawnPromotion, EnPassant, Castle)
/// implement this trait, providing a consistent interface for:
/// - Accessing source and destination squares
/// - Managing move effects (check, checkmate)
/// - Applying and undoing moves on a board
pub trait ChessMoveType {
    /// Returns the square the piece is moving from.
    #[allow(clippy::wrong_self_convention)]
    fn from_square(&self) -> Square;

    /// Returns the square the piece is moving to.
    fn to_square(&self) -> Square;

    /// Returns the effect of this move (check, checkmate, etc.), if calculated.
    fn effect(&self) -> Option<ChessMoveEffect>;

    /// Sets the effect of this move.
    fn set_effect(&mut self, effect: ChessMoveEffect);

    /// Applies this move to the given board.
    ///
    /// This modifies the board state to reflect the move being made,
    /// including updating piece positions, clocks, castling rights, etc.
    fn apply(&self, board: &mut Board) -> Result<(), BoardError>;

    /// Undoes this move on the given board.
    ///
    /// This reverts the board state to before the move was made.
    /// Must be called with the same board state that resulted from `apply`.
    fn undo(&self, board: &mut Board) -> Result<(), BoardError>;
}
