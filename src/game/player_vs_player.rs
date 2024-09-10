use crate::evaluate::GameEnding;
use crate::input_handler;

use super::game::Game;

pub fn player_vs_player() {
    let mut game = Game::new(0);
    loop {
        println!("turn: {}", game.board().turn());
        println!("{}", game.board());

        match game.check_game_over_for_current_turn() {
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

        match command.execute(&mut game) {
            Ok(_chess_move) => {
                game.board_mut().toggle_turn();
                continue;
            }
            Err(error) => println!("error: {}", error),
        }
    }
}
