use super::ChessMove;
use crate::board::square::Square;
use regex::Regex;

impl ChessMove {
    pub fn from_algebraic(algebraic_move: String) -> Result<Self, &'static str> {
        let re = Regex::new("^([a-h][1-8])([a-h][1-8])$").unwrap();
        let caps = match re.captures(&algebraic_move) {
            Some(captures) => captures,
            None => return Err("invalid move"),
        };

        Ok(Self {
            from_square: Square::from_algebraic(&caps[1]),
            to_square: Square::from_algebraic(&caps[2]),
            capture: None,
        })
    }
}
