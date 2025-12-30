//! Play command - play a game against the computer.

use chess::board::color::Color;
use chess::board::Board;
use chess::game::input_source::ConditionalInput;
use chess::game::renderer::TuiRenderer;
use chess::input_handler::fen::STARTING_POSITION_FEN;
use structopt::StructOpt;

use super::util::{create_config, run_game_loop};
use super::Command;

#[derive(StructOpt)]
pub struct PlayArgs {
    #[structopt(short, long, default_value = "4")]
    pub depth: u8,
    #[structopt(short = "c", long = "color", default_value = "random")]
    pub color: Color,
    #[structopt(long = "fen", default_value = STARTING_POSITION_FEN)]
    pub starting_position: Board,
}

impl Command for PlayArgs {
    fn execute(self) {
        let config = create_config(self.depth, self.starting_position);
        let input = ConditionalInput {
            human_color: self.color,
        };

        match TuiRenderer::new(Some(self.color)) {
            Ok(renderer) => {
                run_game_loop(input, renderer, config);
            }
            Err(e) => {
                eprintln!("Failed to initialize TUI: {}", e);
                std::process::exit(1);
            }
        }
    }
}
