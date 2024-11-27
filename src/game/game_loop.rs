use crate::evaluate::GameEnding;
use crate::game::engine::{Engine, EngineConfig};
use crate::game::ui::GameUI;

use super::game_mode::GameMode;

pub struct GameLoop<T: GameMode> {
    engine: Engine,
    ui: GameUI,
    mode: T,
}

impl<T: GameMode> GameLoop<T> {
    pub fn new(mode: T, depth: Option<u8>) -> Self {
        let engine = match depth {
            Some(d) => Engine::with_config(EngineConfig { search_depth: d }),
            None => Engine::new(),
        };

        Self {
            engine,
            ui: GameUI::new(),
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
