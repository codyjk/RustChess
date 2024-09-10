use crate::board::color::Color;
use crate::evaluate::GameEnding;
use crate::game::command::{Command, MakeWaterfallMove};
use crate::game::game::Game;
use crate::game::util::{print_board, print_board_and_stats, print_enter_move_prompt};
use crate::input_handler;
use std::time::SystemTime;
use termion::{clear, cursor};

pub fn play_computer(depth: u8, player_color: Color) {
    let game = &mut Game::new(depth);

    print!("{}{}", cursor::Goto(1, 1), clear::All);
    println!("You are {}", player_color);
    print_board(game.board());
    print_enter_move_prompt();

    loop {
        match game.check_game_over_for_current_turn() {
            Some(GameEnding::Checkmate) => {
                println!("checkmate!");
                break;
            }
            Some(GameEnding::Stalemate) => {
                println!("stalemate!");
                break;
            }
            _ => (),
        };

        // Precalculate the moves and their algebraic notations, so that we
        // can render it after a move is made.
        let enumerated_candidate_moves = game.enumerated_candidate_moves();

        let command: Box<dyn Command> = if player_color == game.board().turn() {
            match input_handler::parse_player_move_input() {
                Ok(command) => command,
                Err(msg) => {
                    println!("{}", msg);
                    continue;
                }
            }
        } else {
            Box::<MakeWaterfallMove>::default()
        };

        let start_time = SystemTime::now();
        match command.execute(game) {
            Ok(_chess_move) => {
                let duration = SystemTime::now().duration_since(start_time).unwrap();
                print!("{}{}", cursor::Goto(1, 1), clear::All);
                game.board_mut().toggle_turn();

                print_board_and_stats(game, enumerated_candidate_moves);
                if player_color == game.board().turn() {
                    println!("* Move took: {:?}", duration);
                    print_enter_move_prompt();
                }
                continue;
            }
            Err(error) => println!("error: {}", error),
        }
    }
}
