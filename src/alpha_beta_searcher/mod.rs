//! Generic alpha-beta search algorithm.

pub mod search;
mod traits;
mod transposition_table;

pub use search::{alpha_beta_search, SearchContext, SearchError};
pub use traits::*;
pub use transposition_table::{BoundType, TranspositionTable};
