use thiserror::Error;

use crate::chess_move::en_passant::EnPassantChessMove;

#[derive(Error, Debug)]
pub enum BoardError {
    #[error("Cannot put a piece on a square that is already occupied")]
    SquareOccupiedBoardPutError,
    #[error("Cannot apply chess move, the `from` square is empty")]
    FromSquareIsEmptyMoveApplicationError,
    #[error("Cannot applychess move, the `to` square is empty")]
    ToSquareIsEmptyMoveApplicationError,
    #[error("Cannot apply chess move, the expected capture result is different than what is on the target square")]
    ToSquareIsEmptyMoveUndoError,
    #[error("Cannot undo chess move, the expected capture result is different than what is on the target square")]
    UnexpectedCaptureResultError,
    #[error("cannot apply en passant, the piece is not a pawn")]
    EnPassantNonPawnMoveApplicationError,
    #[error("Cannot undo en passant, the piece is not a pawn")]
    EnPassantNonPawnMoveUndoError,
    #[error("En passant didn't result in a capture")]
    EnPassantDidNotResultInCaptureError { _chess_move: EnPassantChessMove },
    #[error(
        "Invalid castle move, king can only move 2 squares to left or right on its original rank"
    )]
    InvalidCastleMoveError,
    #[error("Invalid castle state: {msg:?}")]
    InvalidCastleStateError { msg: &'static str },
    #[error("Castle operation was not applied to a king")]
    CastleNonKingError,
    #[error("Castle operation was not applied to a rook")]
    CastleNonRookError,
    #[error("Promotion square did not contain a pawn")]
    PromotionNonPawnError,
    #[error("This move is for a pawn on the final rank, it must be a promotion move")]
    PawnPromotionRequiredError,
    #[error("This pawn is not promotable")]
    PawnNotPromotableError,
}
