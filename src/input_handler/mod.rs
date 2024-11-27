use regex::Regex;
use std::io;
use std::str::FromStr;
use thiserror::Error;

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
        let coord_re = Regex::new("^([a-h][1-8])([a-h][1-8])$").unwrap();
        if let Some(caps) = coord_re.captures(input) {
            return Ok(MoveInput::Coordinate {
                from: caps[1].to_string(),
                to: caps[2].to_string(),
            });
        }

        let alg_re =
            Regex::new("^([NBRQK]?[a-h]?[1-8]?x?[a-h][1-8](=[NBRQ])?[+#]?|O-O(-O)?)$").unwrap();
        if let Some(caps) = alg_re.captures(input) {
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
