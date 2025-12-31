//! Main game loop with state machine and update/render separation
//!
//! The `GameLoop` coordinates the chess game lifecycle using a classic game loop pattern:
//!
//! ## Loop Structure
//!
//! The main loop follows a **Render → Update** pattern:
//! 1. **Render phase**: Display the current game state to the user
//! 2. **Update phase**: Process game logic, handle input, and manage state transitions
//!
//! This ensures the prompt is visible before reading input when the game has ended.
//!
//! ## State Machine
//!
//! The loop uses a simple state machine with two states:
//! - **Playing**: Game is actively being played; accepts move inputs from the input source
//! - **GameEnded**: Game has ended (checkmate/stalemate/draw); accepts restart/switch/exit commands from stdin
//!
//! State transitions occur when:
//! - `Playing` → `GameEnded`: When `engine.check_game_over()` returns a result
//! - `GameEnded` → `Playing`: When user inputs "start over"
//!
//! ## Command Pattern
//!
//! Uses `MoveInput` directly as the command pattern (no redundant wrappers):
//! - **Game moves**: `Coordinate`, `Algebraic`, `UseEngine` → executed during `Playing` state
//! - **Control commands**: `StartOver`, `Exit`, `SwitchGameMode` → handled in `GameEnded` state
//!
//! Commands are mapped to `GameAction` results which indicate loop-level actions (restart, switch mode, exit).
//!
//! ## Input Sources
//!
//! Input handling varies by state:
//! - **Playing**: Uses the `InputSource` trait (e.g., `ConditionalInput`, `EngineInput`, `HumanInput`)
//! - **GameEnded**: Always reads from stdin to allow mode switching in all scenarios (including watch mode)

use crate::board::color::Color;
use crate::chess_move::chess_move::ChessMove;
use crate::game::action::GameAction;
use crate::game::display::GameDisplay;
use crate::game::engine::{Engine, EngineConfig};
use crate::game::input_source::InputSource;
use crate::game::renderer::GameRenderer;
use crate::input_handler::{MenuInput, MoveInput};

/// Current state of the game loop
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameLoopState {
    /// Game is actively being played
    Playing,
    /// Game has ended, waiting for user input
    GameEnded,
}

/// Main game loop coordinating engine, input, and rendering
pub struct GameLoop<I: InputSource, R: GameRenderer> {
    engine: Engine,
    config: EngineConfig,
    ui: GameDisplay,
    input_source: I,
    renderer: R,
    state: GameLoopState,
}

impl<I: InputSource, R: GameRenderer> GameLoop<I, R> {
    pub fn new(input_source: I, renderer: R, config: EngineConfig) -> Self {
        Self {
            engine: Engine::with_config(config.clone()),
            config,
            ui: GameDisplay::new(),
            input_source,
            renderer,
            state: GameLoopState::Playing,
        }
    }

    /// Main game loop following classic update/render pattern
    pub fn run(&mut self) -> GameAction {
        loop {
            self.render();
            if let Some(action) = self.update() {
                return action;
            }
        }
    }

    /// Update phase: processes game logic, input, and state transitions
    /// Returns Some(action) if the game should exit or switch modes
    fn update(&mut self) -> Option<GameAction> {
        match self.state {
            GameLoopState::Playing => self.update_playing(),
            GameLoopState::GameEnded => self.update_game_ended(),
        }
    }

    /// Update logic when game is actively being played
    fn update_playing(&mut self) -> Option<GameAction> {
        if self.engine.check_game_over().is_some() {
            self.state = GameLoopState::GameEnded;
            return None;
        }

        let current_turn = self.engine.board().turn();
        if let Some(input) = self.input_source.get_move(current_turn) {
            self.execute_move_input(input)
        } else {
            eprintln!("Invalid input");
            None
        }
    }

    /// Update logic when game has ended
    fn update_game_ended(&mut self) -> Option<GameAction> {
        match crate::input_handler::parse_menu_input() {
            Ok(MenuInput::StartOver) => {
                self.restart_game();
                None
            }
            Ok(MenuInput::SwitchGameMode { target }) => Some(GameAction::SwitchGameMode { target }),
            Ok(MenuInput::Exit) => Some(GameAction::Exit),
            Err(_) => None, // Invalid input, continue waiting
        }
    }

    fn render(&mut self) {
        let view_model = self.build_view_model();
        self.renderer.render(
            &mut self.ui,
            &self.engine,
            view_model.current_turn,
            view_model.last_move_ref(),
            view_model.game_ending.as_ref(),
        );
    }

    fn build_view_model(&mut self) -> ViewModel {
        let game_ending = self.engine.check_game_over();
        let valid_moves = self.engine.get_valid_moves();
        let current_turn = self.engine.board().turn();
        let last_move = self.find_last_move_with_notation(&valid_moves);

        ViewModel {
            game_ending,
            current_turn,
            last_move,
        }
    }

    fn find_last_move_with_notation(
        &self,
        valid_moves: &[(ChessMove, String)],
    ) -> Option<(ChessMove, String)> {
        self.engine.last_move().and_then(|mv| {
            valid_moves
                .iter()
                .find(|(m, _)| m == &mv)
                .map(|(m, n)| (m.clone(), n.clone()))
        })
    }

    /// Executes a move input and returns an action if needed
    fn execute_move_input(&mut self, input: MoveInput) -> Option<GameAction> {
        match self.engine.make_move_from_input(input) {
            Ok(_) => {
                self.engine.board_mut().toggle_turn();
                self.apply_frame_delay();
                None
            }
            Err(error) => {
                eprintln!("error: {}", error);
                None
            }
        }
    }

    fn restart_game(&mut self) {
        self.engine = Engine::with_config(self.config.clone());
        self.state = GameLoopState::Playing;
    }

    fn apply_frame_delay(&self) {
        if let Some(delay) = self.renderer.frame_delay() {
            std::thread::sleep(delay);
        }
    }
}

/// View model containing all data needed for rendering
struct ViewModel {
    game_ending: Option<crate::evaluate::GameEnding>,
    current_turn: Color,
    last_move: Option<(ChessMove, String)>,
}

impl ViewModel {
    fn last_move_ref(&self) -> Option<(&ChessMove, &str)> {
        self.last_move.as_ref().map(|(m, n)| (m, n.as_str()))
    }
}
