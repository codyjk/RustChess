use crate::evaluate::GameEnding;
use crate::game::display::GameDisplay;
use crate::game::engine::{Engine, EngineConfig};

use super::mode::GameMode;

pub struct GameLoop<T: GameMode> {
    engine: Engine,
    ui: GameDisplay,
    mode: T,
}

impl<T: GameMode> GameLoop<T> {
    pub fn new(mode: T, config: EngineConfig) -> Self {
        Self {
            engine: Engine::with_config(config),
            ui: GameDisplay::new(),
            mode,
        }
    }

    pub fn run(&mut self) {
        loop {
            if let Some(ending) = self.engine.check_game_over() {
                match ending {
                    GameEnding::Checkmate => println!("Checkmate!"),
                    GameEnding::Stalemate => println!("Stalemate!"),
                    GameEnding::Draw => println!("Draw!"),
                }
                break;
            }

            let valid_moves = self.engine.get_valid_moves();
            let current_turn = self.engine.board().turn();
            let last_move = self.engine.last_move().and_then(|mv| {
                valid_moves
                    .iter()
                    .find(|(m, _)| m == &mv)
                    .map(|(m, n)| (m, n.as_str()))
            });

            self.mode
                .render(&mut self.ui, &self.engine, current_turn, last_move);

            match self.mode.get_move(current_turn) {
                Some(input) => match self.engine.make_move_from_input(input) {
                    Ok(_) => {
                        self.engine.board_mut().toggle_turn();
                        if let Some(delay) = self.mode.frame_delay() {
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
