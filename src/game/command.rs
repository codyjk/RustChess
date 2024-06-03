use common::bitboard::bitboard::Bitboard;

use crate::chess_move::ChessMove;

use super::{Game, GameError};

type CommandResult = Result<ChessMove, GameError>;

/// Represents a command that can be executed on a game. This separates the parsing
/// of the command from the execution of the command itself.
pub trait Command {
    fn execute(&self, game: &mut Game) -> CommandResult;
}

/// Represents a command to make a move on the board.
pub struct MakeMove {
    from_square: Bitboard,
    to_square: Bitboard,
}

impl MakeMove {
    pub fn new(from_square: Bitboard, to_square: Bitboard) -> Self {
        Self {
            from_square,
            to_square,
        }
    }
}

impl Command for MakeMove {
    fn execute(&self, game: &mut Game) -> CommandResult {
        game.make_move(self.from_square, self.to_square)
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
