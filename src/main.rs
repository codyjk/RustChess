use std::io;
use std::process;

use chess::board::Board;
use regex::Regex;

fn main() {
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
                process::exit(1);
            }
        };

        match command {
            Command::Quit => {
                println!("exiting...");
                process::exit(0);
            }
            Command::ParseFen { fen } => {
                let board = Board::from_fen(&fen);
                println!("Board:\n{}", board.unwrap().to_ascii());
            }
            Command::UseUCI => {
                println!("id name chess.sh 0.1");
                println!("id author Cody JK");
            }
            Command::RespondWhenReady => {
                println!("readyok");
            }
            Command::Debug { on } => {
                println!("debug mode: {}", on.to_string());
            }
        }
    }
}

pub enum Command {
    ParseFen { fen: String },
    Quit,
    UseUCI,
    Debug { on: bool },
    RespondWhenReady,
}

impl Command {
    pub fn parse(command: &str) -> Result<Command, &'static str> {
        // handle commands with no args
        match command {
            "isready" => return Ok(Command::RespondWhenReady),
            "quit" => return Ok(Command::Quit),
            "uci" => return Ok(Command::UseUCI),
            _ => (),
        };

        // handle commands with args
        if command.starts_with("fen") {
            let re = Regex::new("^fen (.*)$").unwrap();
            let caps = match re.captures(&command) {
                Some(captures) => captures,
                None => return Err("unable to parse fen"),
            };
            return Ok(Command::ParseFen {
                fen: caps[1].to_string(),
            });
        } else if command.starts_with("debug") {
            let re = Regex::new("^debug (on|off)$").unwrap();
            let caps = match re.captures(&command) {
                Some(captures) => captures,
                None => return Err("unable to parse debug command"),
            };
            let on = match &caps[1] {
                "on" => true,
                "off" => false,
                &_ => return Err("unable to parse debug arg (this should never happen since regex only allows on and off)"),
            };
            return Ok(Command::Debug { on });
        }

        return Err("invalid command");
    }
}
