use super::command::{Command, MakeWaterfallMove};
use super::{Game, GameEnding};
use crate::board::color::Color;
use crate::input_handler;
use rand::{self, Rng};
use std::time::SystemTime;
use std::{thread, time};
use termion::clear;

pub fn play_computer(depth: u8) {
    let game = &mut Game::new(depth);
    let rand: u8 = rand::thread_rng().gen();
    let player_color = match rand % 2 {
        0 => Color::White,
        _ => Color::Black,
    };
    println!("{}", clear::All);
    println!("you are {}", player_color);
    loop {
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
            Box::new(MakeWaterfallMove {})
        };

        let start_time = SystemTime::now();
        match command.execute(game) {
            Ok(chessmove) => {
                let duration = SystemTime::now().duration_since(start_time).unwrap();
                println!("{}", clear::All);
                let player = match player_color {
                    c if c == game.board.turn() => "you",
                    _ => "computer",
                };
                game.board.next_turn();
                println!(
                    "{} chose {} (depth={}, took={}ms, halfmove_clock={}, fullmove_clock={}, score={})",
                    player,
                    chessmove,
                    depth,
                    duration.as_millis(),
                    game.board.halfmove_clock(),
                    game.board.fullmove_clock(),
                    game.score(game.board.turn()),
                );
                continue;
            }
            Err(error) => println!("error: {}", error),
        }
    }
}

pub fn computer_vs_computer(move_limit: u8, sleep_between_turns_in_ms: u64) {
    let game = &mut Game::new(2);

    println!("{}", clear::All);

    loop {
        println!("{}", game.board);
        thread::sleep(time::Duration::from_millis(sleep_between_turns_in_ms));

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

        match game.make_waterfall_book_then_alpha_beta_move() {
            Ok(chessmove) => {
                println!("{}", clear::All);
                println!("{} chose {}", game.board.turn(), chessmove);
                game.board.next_turn();
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
            Ok(_chessmove) => {
                game.board.next_turn();
                continue;
            }
            Err(error) => println!("error: {}", error),
        }
    }
}
