//! Generic alpha-beta search algorithm.

mod killer_moves;
pub mod search;
mod traits;
mod transposition_table;

#[cfg(test)]
mod tests;

pub use search::{alpha_beta_search, SearchContext, SearchError};
pub use traits::*;
pub use transposition_table::{BoundType, TranspositionTable};
