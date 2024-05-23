use std::io;

use crate::board::square;
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

pub fn parse_command() -> Result<Box<dyn Command>, InputError> {
    let mut input = String::new();
    let raw = match io::stdin().read_line(&mut input) {
        Ok(_n) => input.trim_start().trim_end(),
        Err(error) => {
            return Err(InputError::IOError {
                error: error.to_string(),
            })
        }
    };

    let re = Regex::new("^([a-h][1-8])([a-h][1-8])$").unwrap();
    let caps = match re.captures(raw) {
        Some(captures) => captures,
        None => {
            return Err(InputError::InvalidInput {
                input: raw.to_string(),
            })
        }
    };

    let command = MakeMove::new(
        square::from_algebraic(&caps[1]),
        square::from_algebraic(&caps[2]),
    );

    Ok(Box::new(command))
}
