use crate::evaluate::GameEnding;
use crate::game::engine::{Engine, EngineConfig};
use crate::game::ui::GameUI;
use crate::input_handler::MoveInput;
use std::thread::sleep;
use std::time::Duration;

pub fn computer_vs_computer(move_limit: u8, sleep_between_turns_in_ms: u64, depth: u8) {
    let engine = &mut Engine::with_config(EngineConfig {
        search_depth: depth,
    });
    let mut ui = GameUI::new();

    loop {
        sleep(Duration::from_millis(sleep_between_turns_in_ms));

        if let Some(ending) = engine.check_game_over() {
            match ending {
                GameEnding::Checkmate => println!("checkmate!"),
                GameEnding::Stalemate => println!("stalemate!"),
                GameEnding::Draw => println!("draw!"),
            }
            break;
        }

        if move_limit > 0 && engine.board().fullmove_clock() > move_limit {
            break;
        }

        let valid_moves = engine.get_valid_moves();
        let current_turn = engine.board().turn();

        match engine.make_move_from_input(MoveInput::UseEngine) {
            Ok(chess_move) => {
                let last_move = valid_moves
                    .iter()
                    .find(|(mv, _)| mv == &chess_move)
                    .map(|(mv, notation)| (mv, notation.as_str()));

                let stats = format!(
                    "* Score: {}\n* Positions searched: {}\n* Search depth: {}",
                    engine
                        .get_search_stats()
                        .last_score
                        .map_or("-".to_string(), |s| s.to_string()),
                    engine.get_search_stats().positions_searched,
                    engine.get_search_stats().depth
                );

                engine.board_mut().toggle_turn();
                engine.clear_cache();
                ui.render_game_state(engine.board(), current_turn, last_move, Some(&stats));
            }
            Err(error) => {
                println!("error: {}", error);
                break;
            }
        }
    }
}
