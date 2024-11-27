use std::thread::sleep;
use std::time::Duration;

use termion::clear;

use crate::evaluate::GameEnding;
use crate::game::engine::{Engine, EngineConfig};
use crate::game::util::print_board_and_stats;
use crate::input_handler::MoveInput;

pub fn computer_vs_computer(move_limit: u8, sleep_between_turns_in_ms: u64, depth: u8) {
    let engine = &mut Engine::with_config(EngineConfig {
        search_depth: depth,
    });

    println!("{}", clear::All);

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
            Ok(_) => {
                println!("{}", clear::All);
                print_board_and_stats(engine, valid_moves, current_turn);
                engine.board_mut().toggle_turn();
                engine.clear_cache();
            }
            Err(error) => {
                println!("error: {}", error);
                break;
            }
        }
    }
}
