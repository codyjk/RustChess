use common::bitboard::square::square_string_to_bitboard;

use crate::chess_move::ChessMove;

use super::{Game, GameError};

type CommandResult = Result<ChessMove, GameError>;

/// Represents a command that can be executed on a game. This separates the parsing
/// of the command from the execution of the command itself.
pub trait Command {
    fn execute(&self, game: &mut Game) -> CommandResult;
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
    fn execute(&self, game: &mut Game) -> CommandResult {
        match self {
            MakeMove::Coordinate {
                from_square,
                to_square,
            } => game.apply_chess_move_by_from_to_coordinates(
                square_string_to_bitboard(from_square),
                square_string_to_bitboard(to_square),
            ),
            MakeMove::Algebraic { algebraic } => {
                game.apply_chess_move_from_raw_algebraic_notation(algebraic.to_string())
            }
        }
    }
}

#[derive(Default)]
/// Represents a command to make a move on the board. The "waterfall" is first
/// an attempt to make a move from the book, then an attempt to choose the "best"
/// move using the alpha-beta minimax algorithm.
pub struct MakeWaterfallMove {}

impl Command for MakeWaterfallMove {
    fn execute(&self, game: &mut Game) -> CommandResult {
        game.make_waterfall_book_then_alpha_beta_move()
    }
}
