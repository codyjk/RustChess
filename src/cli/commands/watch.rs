//! Watch command - watch the computer play against itself.

use std::time::Duration;

use chess::board::Board;
use chess::game::input_source::EngineInput;
use chess::game::renderer::StatsRenderer;
use chess::input_handler::fen::STARTING_POSITION_FEN;
use structopt::StructOpt;

use super::util::{create_config, run_game_loop};
use super::Command;

#[derive(StructOpt)]
pub struct WatchArgs {
    #[structopt(short, long, default_value = "4")]
    pub depth: u8,
    #[structopt(long = "fen", default_value = STARTING_POSITION_FEN)]
    pub starting_position: Board,
}

impl Command for WatchArgs {
    fn execute(self) {
        let config = create_config(self.depth, self.starting_position);
        run_game_loop(
            EngineInput,
            StatsRenderer {
                delay_between_moves: Some(Duration::from_millis(1000)),
            },
            config,
        );
    }
}
