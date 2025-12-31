use crate::board::color::Color;
use crate::input_handler::{InputError, MoveInput};

pub trait InputSource {
    fn get_move(&self, current_turn: Color) -> Result<Option<MoveInput>, InputError>;
}

pub struct HumanInput;

impl InputSource for HumanInput {
    fn get_move(&self, _current_turn: Color) -> Result<Option<MoveInput>, InputError> {
        match crate::input_handler::parse_move_input() {
            Ok(move_input) => Ok(Some(move_input)),
            Err(InputError::UserExit) => Err(InputError::UserExit),
            Err(_) => Ok(None), // Other errors treated as invalid input
        }
    }
}

pub struct EngineInput;

impl InputSource for EngineInput {
    fn get_move(&self, _current_turn: Color) -> Result<Option<MoveInput>, InputError> {
        Ok(Some(MoveInput::UseEngine))
    }
}

pub struct ConditionalInput {
    pub human_color: Color,
}

impl InputSource for ConditionalInput {
    fn get_move(&self, current_turn: Color) -> Result<Option<MoveInput>, InputError> {
        if current_turn == self.human_color {
            match crate::input_handler::parse_move_input() {
                Ok(move_input) => Ok(Some(move_input)),
                Err(InputError::UserExit) => Err(InputError::UserExit),
                Err(_) => Ok(None), // Other errors treated as invalid input
            }
        } else {
            Ok(Some(MoveInput::UseEngine))
        }
    }
}
