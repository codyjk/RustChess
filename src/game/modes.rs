use super::command::MakeAlphaBetaOptimalMove;
use super::Game;
use crate::board::color::Color;
use crate::input_handler;
use rand::{self, Rng};
use termion::clear;

pub fn play_computer(depth: u8) {
    let game = &mut Game::new();
    let rand: u8 = rand::thread_rng().gen();
    let player_color = match rand % 2 {
        0 => Color::White,
        _ => Color::Black,
    };
    println!("{}", clear::All);
    loop {
        println!("{}", game.render_board());
        let command = if player_color == game.turn() {
            match input_handler::parse_command() {
                Ok(command) => command,
                Err(msg) => {
                    println!("{}", msg);
                    continue;
                }
            }
        } else {
            Box::new(MakeAlphaBetaOptimalMove { depth: depth })
        };

        match command.execute(game) {
            Ok(chessmove) => {
                println!("{}", clear::All);
                let player = match player_color {
                    c if c == game.turn() => "you",
                    _ => "computer",
                };
                println!("{} chose {}", player, chessmove);
                game.next_turn();
                continue;
            }
            Err(error) => println!("error: {}", error),
        }
    }
}

pub fn computer_vs_computer() {
    let game = &mut Game::new();
    let mut moves = 0;

    loop {
        println!("{}", game.render_board());
        moves += 1;
        if moves > 250 {
            break;
        }

        match game.make_random_move() {
            Ok(chessmove) => {
                println!("{}", clear::All);
                println!("{} chose {}", game.turn(), chessmove);
                game.next_turn();
                continue;
            }
            Err(error) => {
                println!("error: {}", error);
                break;
            }
        }
    }
}
