//! Move input parsing and validation.

use std::io;
use std::str::FromStr;

use once_cell::sync::Lazy;
use regex::Regex;
use thiserror::Error;

static COORD_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new("^([a-h][1-8])([a-h][1-8])$").expect("COORD_RE regex should be valid"));
static ALG_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new("^([NBRQK]?[a-h]?[1-8]?x?[a-h][1-8](=[NBRQ])?[+#]?|O-O(-O)?)$")
        .expect("ALG_RE regex should be valid")
});

#[derive(Error, Debug)]
pub enum InputError {
    #[error("io error: {error:?}")]
    IOError { error: String },
    #[error("invalid input: {input:?}")]
    InvalidInput { input: String },
}

#[derive(Debug)]
pub enum MoveInput {
    Coordinate { from: String, to: String },
    Algebraic { notation: String },
    UseEngine,
}

impl FromStr for MoveInput {
    type Err = InputError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if let Some(caps) = COORD_RE.captures(input) {
            return Ok(MoveInput::Coordinate {
                from: caps[1].to_string(),
                to: caps[2].to_string(),
            });
        }

        if let Some(caps) = ALG_RE.captures(input) {
            return Ok(MoveInput::Algebraic {
                notation: caps[1].to_string(),
            });
        }

        Err(InputError::InvalidInput {
            input: input.to_string(),
        })
    }
}

pub fn parse_move_input() -> Result<MoveInput, InputError> {
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| InputError::IOError {
            error: e.to_string(),
        })?;

    // Targets `from_str` on the target return type, `MoveInput`
    input.trim().parse()
}
