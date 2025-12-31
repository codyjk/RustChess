//! Watch command - watch the computer play against itself.

use std::time::Duration;

use chess::board::Board;
use chess::game::input_source::EngineInput;
use chess::game::renderer::TuiRenderer;
use chess::input_handler::fen::STARTING_POSITION_FEN;
use structopt::StructOpt;

use super::util::{create_config, run_game_loop};
use super::Command;

#[derive(StructOpt)]
pub struct WatchArgs {
    #[structopt(short, long, default_value = "6")]
    pub depth: u8,
    #[structopt(long = "fen", default_value = STARTING_POSITION_FEN)]
    pub starting_position: Board,
    #[structopt(
        long = "delay",
        default_value = "1000",
        help = "Delay between moves in milliseconds"
    )]
    pub delay_ms: u64,
}

impl Command for WatchArgs {
    fn execute(self) {
        let config = create_config(self.depth, self.starting_position);

        match TuiRenderer::new(None) {
            Ok(renderer) => {
                run_game_loop(EngineInput, renderer, config);

                // Add delay between moves for watch mode
                std::thread::sleep(Duration::from_millis(self.delay_ms));
            }
            Err(e) => {
                eprintln!("Failed to initialize TUI: {}", e);
                std::process::exit(1);
            }
        }
    }
}
