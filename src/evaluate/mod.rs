//! Position evaluation and game state checking.

pub mod evaluation;
pub mod evaluation_tables;

pub use evaluation::{
    board_material_score, current_player_is_in_check, game_ending, is_endgame, player_is_in_check,
    player_is_in_checkmate, score, GameEnding,
};
