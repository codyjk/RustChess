use std::thread::sleep;
use std::time::Duration;

use termion::clear;

use crate::evaluate::GameEnding;
use crate::game::engine::{Engine, EngineConfig};
use crate::game::util::print_board_and_stats;

pub fn computer_vs_computer(move_limit: u8, sleep_between_turns_in_ms: u64, depth: u8) {
    let engine = &mut Engine::with_config(EngineConfig {
        search_depth: depth,
    });

    println!("{}", clear::All);

    loop {
        sleep(Duration::from_millis(sleep_between_turns_in_ms));

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

        if move_limit > 0 && engine.board().fullmove_clock() > move_limit {
            break;
        }

        // Precalculate the moves and their algebraic notations, so that we
        // can render it after a move is made.
        let valid_moves = engine.get_valid_moves();
        let current_turn = engine.board().turn();
        let result = engine.make_best_move();

        match result {
            Ok(_chess_move) => {
                println!("{}", clear::All);
                print_board_and_stats(engine, valid_moves, current_turn);
                engine.board_mut().toggle_turn();
                engine.clear_cache();
                continue;
            }
            Err(error) => {
                println!("error: {}", error);
                break;
            }
        }
    }
}
