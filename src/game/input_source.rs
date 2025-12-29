use crate::board::color::Color;
use crate::input_handler::MoveInput;

pub trait InputSource {
    fn get_move(&self, current_turn: Color) -> Option<MoveInput>;
}

pub struct HumanInput;

impl InputSource for HumanInput {
    fn get_move(&self, _current_turn: Color) -> Option<MoveInput> {
        crate::input_handler::parse_move_input().ok()
    }
}

pub struct EngineInput;

impl InputSource for EngineInput {
    fn get_move(&self, _current_turn: Color) -> Option<MoveInput> {
        Some(MoveInput::UseEngine)
    }
}

pub struct ConditionalInput {
    pub human_color: Color,
}

impl InputSource for ConditionalInput {
    fn get_move(&self, current_turn: Color) -> Option<MoveInput> {
        if current_turn == self.human_color {
            crate::input_handler::parse_move_input().ok()
        } else {
            Some(MoveInput::UseEngine)
        }
    }
}
