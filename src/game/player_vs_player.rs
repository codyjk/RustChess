use crate::evaluate::GameEnding;
use crate::input_handler::parse_move_input;

use super::engine::Engine;

pub fn player_vs_player() {
    let mut engine = Engine::new();
    loop {
        println!("turn: {}", engine.board().turn());
        println!("{}", engine.board());

        if let Some(ending) = engine.check_game_over() {
            match ending {
                GameEnding::Checkmate => println!("Checkmate!"),
                GameEnding::Stalemate => println!("Stalemate!"),
                GameEnding::Draw => println!("Draw!"),
            }
            break;
        }

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
