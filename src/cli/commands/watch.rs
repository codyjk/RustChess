//! Watch command - watch the computer play against itself.

use chess::board::Board;
use chess::input_handler::fen::STARTING_POSITION_FEN;
use structopt::StructOpt;

use super::Command;

#[derive(StructOpt)]
pub struct WatchArgs {
    #[structopt(short, long, default_value = "6")]
    pub depth: u8,
    #[structopt(long = "fen", default_value = STARTING_POSITION_FEN)]
    pub starting_position: Board,
}

impl Command for WatchArgs {
    fn execute(self) {
        use super::util::run_game_with_mode_switching;
        use chess::game::action::GameMode;
        run_game_with_mode_switching(
            GameMode::Watch,
            self.depth,
            chess::board::color::Color::White,
            self.starting_position,
        );
    }
}
