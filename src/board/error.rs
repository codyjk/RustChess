use thiserror::Error;

#[derive(Error, Debug)]
pub enum BoardError {
    #[error("that square already has a piece on it")]
    SquareOccupied,
    #[error("cannot {op:?} chess move, the `from` square is empty")]
    FromSquareIsEmpty { op: &'static str },
    #[error("cannot {op:?} chess move, the `to` square is empty")]
    ToSquareIsEmpty { op: &'static str },
    #[error("the expected capture result is different than what is on the target square")]
    UnexpectedCaptureResult,
    #[error("cannot {op:?} en passant, the piece is not a pawn")]
    EnPassantNonPawn { op: &'static str },
    #[error("en passant didn't result in a capture")]
    EnPassantNonCapture,
    #[error(
        "invalid castle move, king can only move 2 squares to left or right on its original rank"
    )]
    InvalidCastleMoveError,
    #[error("invalid castle state: {msg:?}")]
    InvalidCastleStateError { msg: &'static str },
    #[error("castle operation was not applied to a king")]
    CastleNonKingError,
    #[error("castle operation was not applied to a rook")]
    CastleNonRookError,
}
