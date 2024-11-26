use crate::board::color::Color;
use crate::evaluate::GameEnding;
use crate::game::command::{Command, MakeWaterfallMove};
use crate::game::engine::{Engine, EngineConfig};
use crate::game::util::{print_board, print_board_and_stats, print_enter_move_prompt};
use crate::input_handler;
use std::time::SystemTime;
use termion::{clear, cursor};

pub fn play_computer(depth: u8, player_color: Color) {
    let engine = &mut Engine::with_config(EngineConfig {
        search_depth: depth,
    });

    print!("{}{}", cursor::Goto(1, 1), clear::All);
    println!("You are {}", player_color);
    print_board(engine.board());
    print_enter_move_prompt();

    loop {
        match engine.check_game_over() {
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
        let valid_moves = engine.get_valid_moves();
        let current_turn = engine.board().turn();

        let command: Box<dyn Command> = if player_color == engine.board().turn() {
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
        match command.execute(engine) {
            Ok(_chess_move) => {
                let duration = SystemTime::now().duration_since(start_time).unwrap();
                print!("{}{}", cursor::Goto(1, 1), clear::All);
                engine.board_mut().toggle_turn();

                print_board_and_stats(engine, valid_moves, current_turn);
                if player_color == engine.board().turn() {
                    println!("* Move took: {:?}", duration);
                    print_enter_move_prompt();
                }
                continue;
            }
            Err(error) => println!("error: {}", error),
        }
    }
}
