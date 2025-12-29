//! PvP command - play a game against another human.

use chess::board::Board;
use chess::game::input_source::HumanInput;
use chess::game::renderer::SimpleRenderer;
use chess::input_handler::fen::STARTING_POSITION_FEN;
use structopt::StructOpt;

use super::util::{create_config, run_game_loop};
use super::Command;

#[derive(StructOpt)]
pub struct PvpArgs {
    #[structopt(long = "fen", default_value = STARTING_POSITION_FEN)]
    pub starting_position: Board,
}

impl Command for PvpArgs {
    fn execute(self) {
        let config = create_config(0, self.starting_position);
        run_game_loop(HumanInput, SimpleRenderer, config);
    }
}
