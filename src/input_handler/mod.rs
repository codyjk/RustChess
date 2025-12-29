//! Input parsing and handling for chess moves and positions.

pub mod fen;
pub mod input;

pub use input::{parse_move_input, InputError, MoveInput};
