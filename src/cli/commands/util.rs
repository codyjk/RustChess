//! Shared utilities for CLI commands.

use chess::board::Board;
use chess::game::engine::EngineConfig;
use chess::game::input_source::InputSource;
use chess::game::r#loop::GameLoop;
use chess::game::renderer::GameRenderer;

pub(crate) fn run_game_loop<I, R>(input_source: I, renderer: R, config: EngineConfig)
where
    I: InputSource,
    R: GameRenderer,
{
    let mut game = GameLoop::new(input_source, renderer, config);
    game.run();
}

pub(crate) fn create_config(depth: u8, starting_position: Board) -> EngineConfig {
    EngineConfig {
        search_depth: depth,
        starting_position,
    }
}
