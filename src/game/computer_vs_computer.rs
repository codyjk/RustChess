use std::thread::sleep;
use std::time::Duration;

use termion::clear;

use crate::evaluate::GameEnding;
use crate::game::game::Game;
use crate::game::util::print_board_and_stats;

pub fn computer_vs_computer(move_limit: u8, sleep_between_turns_in_ms: u64, depth: u8) {
    let mut game = Game::new(depth);

    println!("{}", clear::All);

    loop {
        sleep(Duration::from_millis(sleep_between_turns_in_ms));

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

        if move_limit > 0 && game.fullmove_clock() > move_limit {
            break;
        }

        // Precalculate the moves and their algebraic notations, so that we
        // can render it after a move is made.
        let enumerated_candidate_moves = game.enumerated_candidate_moves();
        let current_turn = game.board().turn();

        let result = game.make_waterfall_book_then_alpha_beta_move();

        match result {
            Ok(_chess_move) => {
                println!("{}", clear::All);
                print_board_and_stats(&mut game, enumerated_candidate_moves, current_turn);
                game.board_mut().toggle_turn();
                game.reset_move_generator_cache_hit_count();
                continue;
            }
            Err(error) => {
                println!("error: {}", error);
                break;
            }
        }
    }
}
