use common::bitboard::square::square_string_to_bitboard;

use crate::chess_move::chess_move::ChessMove;

use super::engine::{Engine, EngineError};

type CommandResult = Result<ChessMove, EngineError>;

/// Represents a command that can be executed on a engine. This separates the parsing
/// of the command from the execution of the command itself.
pub trait Command {
    fn execute(&self, engine: &mut Engine) -> CommandResult;
}

/// Represents a command to make a move on the board.
pub enum MakeMove {
    /// Represents a move based on the from and to coordinates, e.g. "e2e4".
    Coordinate {
        from_square: String,
        to_square: String,
    },

    /// Represents a move in algebraic notation, e.g. "e4".
    Algebraic { algebraic: String },
}

impl Command for MakeMove {
    fn execute(&self, engine: &mut Engine) -> CommandResult {
        match self {
            MakeMove::Coordinate {
                from_square,
                to_square,
            } => engine.make_move_by_squares(
                square_string_to_bitboard(from_square),
                square_string_to_bitboard(to_square),
            ),
            MakeMove::Algebraic { algebraic } => engine.make_move_algebraic(algebraic.to_string()),
        }
    }
}

#[derive(Default)]
/// Represents a command to make a move on the board. The "waterfall" is first
/// an attempt to make a move from the book, then an attempt to choose the "best"
/// move using the alpha-beta minimax algorithm.
pub struct MakeWaterfallMove {}

impl Command for MakeWaterfallMove {
    fn execute(&self, engine: &mut Engine) -> CommandResult {
        engine.make_best_move()
    }
}
