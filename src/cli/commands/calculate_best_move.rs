//! Calculate best move command - determine the best move from a position.

use chess::board::Board;
use chess::game::engine::{Engine, EngineConfig};
use structopt::StructOpt;

use super::Command;

#[derive(StructOpt)]
pub struct CalculateBestMoveArgs {
    #[structopt(short, long, default_value = "4")]
    pub depth: u8,
    #[structopt(long = "fen")]
    pub starting_position: Board,
}

impl Command for CalculateBestMoveArgs {
    fn execute(self) {
        let config = EngineConfig {
            search_depth: self.depth,
            starting_position: self.starting_position,
        };
        let mut engine = Engine::with_config(config);

        let valid_moves = engine.get_valid_moves();
        if valid_moves.is_empty() {
            eprintln!("There are no valid moves in the given position.");
            return;
        }

        match engine.get_best_move() {
            Ok(best_move) => {
                let algebraic_move = valid_moves
                    .iter()
                    .find_map(|(chess_move, algebraic_notation)| {
                        (chess_move == &best_move).then_some(algebraic_notation.as_str())
                    })
                    .expect("best move should be in valid moves");
                println!("{}", algebraic_move);
            }
            Err(err) => eprintln!("Failed to calculate best move: {}", err),
        }
    }
}
