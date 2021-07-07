use std::io;
use std::process;

use chess::bitboard::{Bitboard, ChessMove};
use regex::Regex;

fn main() {
    let mut board = Bitboard::starting_position();

    println!("{}", board.to_ascii());

    loop {
        let mut input = String::new();

        let parsed = match io::stdin().read_line(&mut input) {
            Ok(_n) => Command::parse(&input.trim_start().trim_end()),
            Err(error) => {
                println!("error: {}", error);
                continue;
            }
        };

        let command = match parsed {
            Ok(cmd) => cmd,
            Err(error) => {
                println!("failed to parse command `{}`: {}", input.trim_end(), error);
                continue;
            }
        };

        match command {
            Command::Move { algebraic_move } => {
                let chessmove = match ChessMove::from_algebraic(algebraic_move) {
                    Ok(result) => result,
                    Err(error) => {
                        println!("move error: {}", error);
                        continue;
                    }
                };
                let result = board.apply(chessmove);
                let captured_piece = match result {
                    Ok(piece) => piece,
                    Err(error) => {
                        println!("move error: {}", error);
                        continue;
                    }
                };

                match captured_piece {
                    Some((piece, color)) => println!(
                        "captured {} on {}",
                        piece.to_fen(color),
                        chessmove.to_square.to_algebraic()
                    ),
                    _ => (),
                };

                println!("{}", board.to_ascii());
            }
            Command::Quit => process::exit(0),
        }
    }
}

pub enum Command {
    Move { algebraic_move: String },
    Quit,
}

impl Command {
    pub fn parse(command: &str) -> Result<Command, &'static str> {
        // handle commands with no args
        match command {
            "quit" => return Ok(Command::Quit),
            _ => (),
        };

        // handle commands with args
        if command.starts_with("move") {
            let re = Regex::new("^move (.*)$").unwrap();
            let caps = match re.captures(&command) {
                Some(captures) => captures,
                None => return Err("unable to parse move command"),
            };
            return Ok(Command::Move {
                algebraic_move: caps[1].to_string(),
            });
        }

        return Err("invalid command");
    }
}
