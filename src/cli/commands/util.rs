//! Shared utilities for CLI commands.

use chess::board::color::Color;
use chess::board::Board;
use chess::game::action::{GameAction, GameMode};
use chess::game::engine::EngineConfig;
use chess::game::input_source::{ConditionalInput, EngineInput, HumanInput, InputSource};
use chess::game::r#loop::GameLoop;
use chess::game::renderer::GameRenderer;
use chess::game::renderer::TuiRenderer;

pub(crate) fn run_game_loop<I, R>(input_source: I, renderer: R, config: EngineConfig) -> GameAction
where
    I: InputSource,
    R: GameRenderer,
{
    let mut game = GameLoop::new(input_source, renderer, config);
    game.run()
}

pub(crate) fn create_config(depth: u8, starting_position: Board) -> EngineConfig {
    EngineConfig {
        search_depth: depth,
        starting_position,
    }
}

/// Unified game runner that can switch between modes
pub(crate) fn run_game_with_mode_switching(
    initial_mode: GameMode,
    default_depth: u8,
    default_color: Color,
    starting_position: Board,
) {
    let mut current_mode = initial_mode;
    let current_depth = default_depth;
    let current_color = default_color;
    let starting_position_clone = starting_position.clone();
    let mut current_position = starting_position;

    loop {
        let action = match current_mode {
            GameMode::Play => {
                let config = create_config(current_depth, current_position);
                let input = ConditionalInput {
                    human_color: current_color,
                };

                match TuiRenderer::new(Some(current_color)) {
                    Ok(renderer) => run_game_loop(input, renderer, config),
                    Err(e) => {
                        eprintln!("Failed to initialize TUI: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            GameMode::Watch => {
                let config = create_config(current_depth, current_position);
                let input = EngineInput;

                match TuiRenderer::new(None) {
                    Ok(renderer) => run_game_loop(input, renderer, config),
                    Err(e) => {
                        eprintln!("Failed to initialize TUI: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            GameMode::Pvp => {
                let config = create_config(0, current_position);
                let input = HumanInput;

                match TuiRenderer::new(None) {
                    Ok(renderer) => run_game_loop(input, renderer, config),
                    Err(e) => {
                        eprintln!("Failed to initialize TUI: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        };

        match action {
            GameAction::RestartSameMode => {
                current_position = starting_position_clone.clone();
                continue;
            }
            GameAction::SwitchGameMode { target } => {
                current_mode = target;
                current_position = starting_position_clone.clone();
                continue;
            }
            GameAction::Exit => {
                break;
            }
        }
    }
}
