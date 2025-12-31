//! PvP command - play a game against another human.

use chess::board::Board;
use chess::input_handler::fen::STARTING_POSITION_FEN;
use structopt::StructOpt;

use super::Command;

#[derive(StructOpt)]
pub struct PvpArgs {
    #[structopt(long = "fen", default_value = STARTING_POSITION_FEN)]
    pub starting_position: Board,
}

impl Command for PvpArgs {
    fn execute(self) {
        use super::util::run_game_with_mode_switching;
        use chess::game::action::GameMode;
        run_game_with_mode_switching(
            GameMode::Pvp,
            0,                                 // Depth not used in PvP
            chess::board::color::Color::White, // Not used in PvP
            self.starting_position,
        );
    }
}
