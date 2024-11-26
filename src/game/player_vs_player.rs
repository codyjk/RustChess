use crate::evaluate::GameEnding;
use crate::input_handler;

use super::engine::Engine;

pub fn player_vs_player() {
    let mut engine = Engine::new();
    loop {
        println!("turn: {}", engine.board().turn());
        println!("{}", engine.board());

        match engine.check_game_over() {
            Some(GameEnding::Checkmate) => {
                println!("checkmate!");
                break;
            }
            Some(GameEnding::Stalemate) => {
                println!("stalemate!");
                break;
            }
            Some(GameEnding::Draw) => {
                println!("draw!");
                break;
            }
            _ => (),
        };

        let command = match input_handler::parse_player_move_input() {
            Ok(command) => command,
            Err(msg) => {
                println!("{}", msg);
                continue;
            }
        };

        match command.execute(&mut engine) {
            Ok(_chess_move) => {
                engine.board_mut().toggle_turn();
                continue;
            }
            Err(error) => println!("error: {}", error),
        }
    }
}
