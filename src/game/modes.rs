use super::{Game, GameEnding};
use crate::board::color::Color;
use crate::game::command::{Command, MakeWaterfallMove};
use crate::input_handler;
use std::thread::sleep;
use std::time::{Duration, SystemTime};
use termion::clear;

// TODO(codyjk): This code, and the resulting UI, needs to be made prettier.

pub fn play_computer(depth: u8, player_color: Color) {
    let game = &mut Game::new(depth);

    println!("{}", clear::All);
    println!("You are {}", player_color);
    println!("{}", game.board);
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

        let command: Box<dyn Command> = if player_color == game.board.turn() {
            match input_handler::parse_command() {
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
                println!("{}", clear::All);
                game.board.toggle_turn();

                print_board_and_stats(game);
                if player_color == game.board.turn() {
                    print_enter_move_prompt();
                } else {
                    println!("* Move took: {:?}", duration);
                }
                continue;
            }
            Err(error) => println!("error: {}", error),
        }
    }
}

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

        let result = game.make_waterfall_book_then_alpha_beta_move();

        match result {
            Ok(_chess_move) => {
                println!("{}", clear::All);
                game.board.toggle_turn();
                print_board_and_stats(&mut game);
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

pub fn player_vs_player() {
    let game = &mut Game::new(0);
    loop {
        println!("turn: {}", game.board.turn());
        println!("{}", game.board);

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

        let command = match input_handler::parse_command() {
            Ok(command) => command,
            Err(msg) => {
                println!("{}", msg);
                continue;
            }
        };

        match command.execute(game) {
            Ok(_chess_move) => {
                game.board.toggle_turn();
                continue;
            }
            Err(error) => println!("error: {}", error),
        }
    }
}

fn print_board_and_stats(game: &mut Game) {
    let last_move = match game.last_move() {
        Some(chess_move) => chess_move.to_string(),
        None => "-".to_string(),
    };
    println!(
        "{} chose move: {}\n",
        game.board.turn().opposite(),
        last_move
    );
    println!("{}\n", game.board);
    println!("* Turn: {}", game.board.turn());
    println!("* Halfmove clock: {}", game.board.halfmove_clock());
    println!("* Fullmove clock: {}", game.board.fullmove_clock());
    println!("* Score: {}", game.score(game.board.turn()));
    println!(
        "* Positions searched: {} (depth {})",
        game.searched_position_count(),
        game.search_depth()
    );
}

fn print_enter_move_prompt() {
    println!("Enter your move:");
}
