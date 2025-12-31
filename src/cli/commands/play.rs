//! Play command - play a game against the computer.

use chess::board::color::Color;
use chess::board::Board;
use chess::input_handler::fen::STARTING_POSITION_FEN;
use structopt::StructOpt;

use super::Command;

#[derive(StructOpt)]
pub struct PlayArgs {
    #[structopt(short, long, default_value = "6")]
    pub depth: u8,
    #[structopt(short = "c", long = "color", default_value = "random")]
    pub color: Color,
    #[structopt(long = "fen", default_value = STARTING_POSITION_FEN)]
    pub starting_position: Board,
}

impl Command for PlayArgs {
    fn execute(self) {
        use super::util::run_game_with_mode_switching;
        use chess::game::action::GameMode;
        run_game_with_mode_switching(
            GameMode::Play,
            self.depth,
            self.color,
            self.starting_position,
        );
    }
}
