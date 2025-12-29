//! Opening book for chess positions.

#[allow(clippy::module_inception)]
pub mod book;

pub use book::{Book, BookMove, BookNode, OpeningLine};
