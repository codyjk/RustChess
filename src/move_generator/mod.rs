//! Chess move generation algorithms.

pub mod generator;
mod magic_table;
pub mod targets;

pub use generator::{ChessMoveList, MoveGenerator, PAWN_PROMOTIONS};
pub use targets::{PieceTarget, PieceTargetList, Targets};
