use crate::evaluate::GameEnding;
use crate::game::ui::GameUI;
use crate::input_handler::parse_move_input;

use super::engine::Engine;

pub fn player_vs_player() {
    let mut engine = Engine::new();
    let mut ui = GameUI::new();

    loop {
        if let Some(ending) = engine.check_game_over() {
            match ending {
                GameEnding::Checkmate => println!("Checkmate!"),
                GameEnding::Stalemate => println!("Stalemate!"),
                GameEnding::Draw => println!("Draw!"),
            }
            break;
        }

        let valid_moves = engine.get_valid_moves();
        let current_turn = engine.board().turn();
        let last_move = engine.last_move().and_then(|mv| {
            valid_moves
                .iter()
                .find(|(m, _)| m == &mv)
                .map(|(m, n)| (m, n.as_str()))
        });

        ui.render_game_state(engine.board(), current_turn, last_move, None);
        println!("Enter your move:");

        match parse_move_input() {
            Ok(input) => match engine.make_move_from_input(input) {
                Ok(_) => {
                    engine.board_mut().toggle_turn();
                }
                Err(error) => println!("error: {}", error),
            },
            Err(msg) => println!("{}", msg),
        }
    }
}
