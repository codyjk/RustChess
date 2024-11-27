use crate::board::color::Color;
use crate::evaluate::GameEnding;
use crate::game::engine::{Engine, EngineConfig};
use crate::game::util::{print_board, print_board_and_stats, print_enter_move_prompt};
use crate::input_handler::{parse_move_input, MoveInput};
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
            Ok(_) => {
                let duration = SystemTime::now().duration_since(start_time).unwrap();
                print!("{}{}", cursor::Goto(1, 1), clear::All);
                engine.board_mut().toggle_turn();

                print_board_and_stats(engine, valid_moves, current_turn);
                if player_color == engine.board().turn() {
                    println!("* Move took: {:?}", duration);
                    print_enter_move_prompt();
                }
            }
            Err(error) => println!("error: {}", error),
        }
    }
}
