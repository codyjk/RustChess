use crate::game::display::GameDisplay;
use crate::game::engine::{Engine, EngineConfig};
use crate::game::input_source::InputSource;
use crate::game::renderer::GameRenderer;

pub struct GameLoop<I: InputSource, R: GameRenderer> {
    engine: Engine,
    ui: GameDisplay,
    input_source: I,
    renderer: R,
}

impl<I: InputSource, R: GameRenderer> GameLoop<I, R> {
    pub fn new(input_source: I, renderer: R, config: EngineConfig) -> Self {
        Self {
            engine: Engine::with_config(config),
            ui: GameDisplay::new(),
            input_source,
            renderer,
        }
    }

    pub fn run(&mut self) {
        loop {
            // Check for game ending before processing moves
            let game_ending = self.engine.check_game_over();

            let valid_moves = self.engine.get_valid_moves();
            let current_turn = self.engine.board().turn();
            let last_move = self.engine.last_move().and_then(|mv| {
                valid_moves
                    .iter()
                    .find(|(m, _)| m == &mv)
                    .map(|(m, n)| (m, n.as_str()))
            });

            // Render the current position (including final position if game ended)
            self.renderer.render(
                &mut self.ui,
                &self.engine,
                current_turn,
                last_move,
                game_ending.as_ref(),
            );

            // If game ended, break after rendering
            if game_ending.is_some() {
                break;
            }

            match self.input_source.get_move(current_turn) {
                Some(input) => match self.engine.make_move_from_input(input) {
                    Ok(_) => {
                        self.engine.board_mut().toggle_turn();
                        if let Some(delay) = self.renderer.frame_delay() {
                            std::thread::sleep(delay);
                        }
                    }
                    Err(error) => println!("error: {}", error),
                },
                None => println!("Invalid input"),
            }
        }
    }
}
