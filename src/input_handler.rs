use std::io;

use crate::board::square;
use crate::game::command::{Command, MakeMove};
use regex::Regex;

pub fn parse_command() -> Result<Box<dyn Command>, String> {
    let mut input = String::new();
    let raw = match io::stdin().read_line(&mut input) {
        Ok(_n) => input.trim_start().trim_end(),
        Err(error) => return Err(format!("io error: {}", error)),
    };

    let re = Regex::new("^([a-h][1-8])([a-h][1-8])$").unwrap();
    let caps = match re.captures(&raw) {
        Some(captures) => captures,
        None => return Err(format!("invalid input: {}", raw)),
    };

    let command = MakeMove::new(
        square::from_algebraic(&caps[1]),
        square::from_algebraic(&caps[2]),
    );

    Ok(Box::new(command))
}
