//! Chess-specific implementation of the alpha-beta search traits.

pub mod implementation;
mod move_orderer;

#[cfg(test)]
mod tests;

pub use implementation::{search_best_move, ChessEvaluator, ChessMoveGenerator};
pub use move_orderer::ChessMoveOrderer;
