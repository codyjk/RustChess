use crate::{bitboard::bitboard::Bitboard, chess_move::ChessMove};

use super::{Game, GameError};

type CommandResult = Result<ChessMove, GameError>;

pub trait Command {
    fn execute(&self, game: &mut Game) -> CommandResult;
}

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

pub struct MakeWaterfallMove {}

impl Command for MakeWaterfallMove {
    fn execute(&self, game: &mut Game) -> CommandResult {
        game.make_waterfall_book_then_alpha_beta_move()
    }
}
