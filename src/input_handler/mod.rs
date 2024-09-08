use std::io;

use crate::game::command::{Command, MakeMove};
use regex::Regex;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InputError {
    #[error("io error: {error:?}")]
    IOError { error: String },
    #[error("invalid input: {input:?}")]
    InvalidInput { input: String },
}

pub fn parse_player_move_input() -> Result<Box<dyn Command>, InputError> {
    let mut input = String::new();
    let raw = match io::stdin().read_line(&mut input) {
        Ok(_n) => input.trim_start().trim_end(),
        Err(error) => {
            return Err(InputError::IOError {
                error: error.to_string(),
            })
        }
    };

    let coordinate_re = Regex::new("^([a-h][1-8])([a-h][1-8])$").unwrap();
    let algebraic_re = Regex::new("^([NBRQK]?[a-h]?[1-8]?x?[a-h][1-8](=[NBRQ])?[+#]?)$").unwrap();

    if let Some(caps) = coordinate_re.captures(raw) {
        let from_square = caps.get(1).unwrap().as_str();
        let to_square = caps.get(2).unwrap().as_str();
        let command = MakeMove::Coordinate {
            from_square: from_square.to_string(),
            to_square: to_square.to_string(),
        };

        Ok(Box::new(command))
    } else if let Some(caps) = algebraic_re.captures(raw) {
        let algebraic = caps.get(1).unwrap().as_str().to_string();
        let command = MakeMove::Algebraic { algebraic };

        Ok(Box::new(command))
    } else {
        Err(InputError::InvalidInput {
            input: raw.to_string(),
        })
    }
}
