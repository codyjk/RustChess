//! Chess-specific implementation of the alpha-beta search traits.

pub mod history_table;
pub mod implementation;
mod move_orderer;

#[cfg(test)]
mod tests;

pub use history_table::HistoryTable;
pub use implementation::{
    search_best_move, search_best_move_with_history, ChessEvaluator, ChessMoveGenerator,
};
pub use move_orderer::ChessMoveOrderer;
