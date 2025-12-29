//! Chess move generation algorithms.

pub mod generator;
mod magic_table;
mod targets;

pub use generator::{ChessMoveList, MoveGenerator, PAWN_PROMOTIONS};
