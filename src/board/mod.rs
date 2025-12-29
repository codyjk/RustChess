//! Chess board state representation and management.

#[allow(clippy::module_inception)]
pub mod board;
pub mod castle_rights;
pub mod castle_rights_bitmask;
pub mod color;
pub mod error;
pub mod fullmove_number;
pub mod halfmove_clock;
pub mod piece;

mod display;
mod move_info;
mod piece_set;
mod position_info;
mod state_stack;

pub use board::Board;
pub use color::Color;
pub use piece::Piece;
