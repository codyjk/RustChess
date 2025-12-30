//! CLI command implementations.

pub trait Command {
    fn execute(self);
}

pub mod benchmark_alpha_beta;
pub mod calculate_best_move;
pub mod count_positions;
pub mod determine_stockfish_elo;
pub mod play;
pub mod pvp;
pub mod uci;
pub mod watch;

// Shared utilities for commands
pub(crate) mod util;
