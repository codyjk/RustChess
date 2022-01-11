use super::{Game, GameError};
use crate::moves::chess_move::ChessMove;

type CommandResult = Result<ChessMove, GameError>;

pub trait Command {
    fn execute(&self, game: &mut Game) -> CommandResult;
}

pub struct MakeMove {
    from_square: u64,
    to_square: u64,
}

impl MakeMove {
    pub fn new(from_square: u64, to_square: u64) -> Self {
        Self {
            from_square: from_square,
            to_square: to_square,
        }
    }
}

impl Command for MakeMove {
    fn execute(&self, game: &mut Game) -> CommandResult {
        game.make_move(self.from_square, self.to_square)
    }
}

pub struct MakeRandomMove {}

impl Command for MakeRandomMove {
    fn execute(&self, game: &mut Game) -> CommandResult {
        game.make_random_move()
    }
}

pub struct MakeOptimalMove {
    pub depth: u8,
}
impl Command for MakeOptimalMove {
    fn execute(&self, game: &mut Game) -> CommandResult {
        game.make_alpha_beta_best_move(self.depth)
    }
}

pub struct MakeWaterfallMove {
    pub depth: u8,
}
impl Command for MakeWaterfallMove {
    fn execute(&self, game: &mut Game) -> CommandResult {
        game.make_waterfall_book_then_alpha_beta_move(self.depth)
    }
}
