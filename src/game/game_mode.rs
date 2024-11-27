use crate::board::color::Color;
use crate::chess_move::chess_move::ChessMove;
use crate::game::engine::Engine;
use crate::game::ui::GameUI;
use crate::input_handler::{parse_move_input, MoveInput};
use std::time::Duration;

pub trait GameMode {
    fn get_move(&self, current_turn: Color) -> Option<MoveInput>;
    fn render(
        &self,
        ui: &mut GameUI,
        engine: &Engine,
        current_turn: Color,
        last_move: Option<(&ChessMove, &str)>,
    );
    fn frame_delay(&self) -> Option<Duration>;
}

pub struct HumanVsComputer {
    pub human_color: Color,
}

pub struct ComputerVsComputer {
    pub depth: u8,
    pub delay: Option<Duration>,
}

pub struct HumanVsHuman;

impl GameMode for HumanVsComputer {
    fn get_move(&self, current_turn: Color) -> Option<MoveInput> {
        if current_turn == self.human_color {
            parse_move_input().ok()
        } else {
            Some(MoveInput::UseEngine)
        }
    }

    fn render(
        &self,
        ui: &mut GameUI,
        engine: &Engine,
        current_turn: Color,
        last_move: Option<(&ChessMove, &str)>,
    ) {
        let stats = format!(
            "* Score: {}\n* Positions searched: {}\n* Search depth: {}",
            engine
                .get_search_stats()
                .last_score
                .map_or("-".to_string(), |s| s.to_string()),
            engine.get_search_stats().positions_searched,
            engine.get_search_stats().depth,
        );
        ui.render_game_state(engine.board(), current_turn, last_move, Some(&stats));
        if current_turn == self.human_color {
            println!("Enter your move:");
        }
    }

    fn frame_delay(&self) -> Option<Duration> {
        None
    }
}

impl GameMode for ComputerVsComputer {
    fn get_move(&self, _current_turn: Color) -> Option<MoveInput> {
        Some(MoveInput::UseEngine)
    }

    fn render(
        &self,
        ui: &mut GameUI,
        engine: &Engine,
        current_turn: Color,
        last_move: Option<(&ChessMove, &str)>,
    ) {
        let stats = format!(
            "* Score: {}\n* Positions searched: {}\n* Search depth: {}",
            engine
                .get_search_stats()
                .last_score
                .map_or("-".to_string(), |s| s.to_string()),
            engine.get_search_stats().positions_searched,
            engine.get_search_stats().depth,
        );
        ui.render_game_state(engine.board(), current_turn, last_move, Some(&stats));
    }

    fn frame_delay(&self) -> Option<Duration> {
        self.delay
    }
}

impl GameMode for HumanVsHuman {
    fn get_move(&self, _current_turn: Color) -> Option<MoveInput> {
        parse_move_input().ok()
    }

    fn render(
        &self,
        ui: &mut GameUI,
        engine: &Engine,
        current_turn: Color,
        last_move: Option<(&ChessMove, &str)>,
    ) {
        ui.render_game_state(engine.board(), current_turn, last_move, None);
        println!("Enter your move:");
    }

    fn frame_delay(&self) -> Option<Duration> {
        None
    }
}
