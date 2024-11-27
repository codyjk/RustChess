use crate::board::color::Color;
use crate::evaluate::GameEnding;
use crate::game::engine::{Engine, EngineConfig};
use crate::game::ui::GameUI;
use crate::input_handler::{parse_move_input, MoveInput};
use std::time::SystemTime;

pub fn play_computer(depth: u8, player_color: Color) {
    let engine = &mut Engine::with_config(EngineConfig {
        search_depth: depth,
    });
    let mut ui = GameUI::new();

    ui.render_game_state(engine.board(), engine.board().turn(), None, None);
    println!("Enter your move:");

    loop {
        if let Some(ending) = engine.check_game_over() {
            match ending {
                GameEnding::Checkmate => println!("checkmate!"),
                GameEnding::Stalemate => println!("stalemate!"),
                _ => (),
            }
            break;
        }

        let valid_moves = engine.get_valid_moves();
        let current_turn = engine.board().turn();

        let input = if player_color == engine.board().turn() {
            match parse_move_input() {
                Ok(input) => input,
                Err(msg) => {
                    println!("{}", msg);
                    continue;
                }
            }
        } else {
            MoveInput::UseEngine
        };

        let start_time = SystemTime::now();
        match engine.make_move_from_input(input) {
            Ok(chess_move) => {
                let duration = SystemTime::now().duration_since(start_time).unwrap();
                let last_move = valid_moves
                    .iter()
                    .find(|(mv, _)| mv == &chess_move)
                    .map(|(mv, notation)| (mv, notation.as_str()));

                let stats =
                    format!(
                    "* Score: {}\n* Positions searched: {}\n* Search depth: {}\n* Move took: {:?}",
                    engine.get_search_stats().last_score.map_or("-".to_string(), |s| s.to_string()),
                    engine.get_search_stats().positions_searched,
                    engine.get_search_stats().depth,
                    duration
                );

                engine.board_mut().toggle_turn();
                ui.render_game_state(engine.board(), current_turn, last_move, Some(&stats));

                if player_color == engine.board().turn() {
                    println!("Enter your move:");
                }
            }
            Err(error) => println!("error: {}", error),
        }
    }
}
