use crate::board::color::Color;
use crate::chess_move::chess_move::ChessMove;
use crate::game::display::GameDisplay;
use crate::game::engine::Engine;
use crate::input_handler::{parse_move_input, MoveInput};
use std::time::Duration;

pub trait GameMode {
    fn get_move(&self, current_turn: Color) -> Option<MoveInput>;
    fn render(
        &self,
        ui: &mut GameDisplay,
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
    /// The engine can calculate moves very quickly, so adding a slight delay
    /// between moves makes the game easier to observe.
    pub delay_between_moves: Option<Duration>,
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
        ui: &mut GameDisplay,
        engine: &Engine,
        current_turn: Color,
        last_move: Option<(&ChessMove, &str)>,
    ) {
        let stats = engine.get_search_stats();
        let stats_display = format!(
            "* Score: {}\n* Positions searched: {} (depth: {})\n* Move took: {}",
            stats.last_score.map_or("-".to_string(), |s| s.to_string()),
            stats.positions_searched,
            stats.depth,
            stats
                .last_search_duration
                .map_or("-".to_string(), |d| format!("{:?}", d))
        );
        ui.render_game_state(
            engine.board(),
            current_turn,
            last_move,
            Some(&stats_display),
        );
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
        ui: &mut GameDisplay,
        engine: &Engine,
        current_turn: Color,
        last_move: Option<(&ChessMove, &str)>,
    ) {
        let stats = engine.get_search_stats();
        let stats_display = format!(
            "* Score: {}\n* Positions searched: {} (depth: {})\n* Move took: {}",
            stats.last_score.map_or("-".to_string(), |s| s.to_string()),
            stats.positions_searched,
            stats.depth,
            stats
                .last_search_duration
                .map_or("-".to_string(), |d| format!("{:?}", d))
        );
        ui.render_game_state(
            engine.board(),
            current_turn,
            last_move,
            Some(&stats_display),
        );
    }

    fn frame_delay(&self) -> Option<Duration> {
        self.delay_between_moves
    }
}

impl GameMode for HumanVsHuman {
    fn get_move(&self, _current_turn: Color) -> Option<MoveInput> {
        parse_move_input().ok()
    }

    fn render(
        &self,
        ui: &mut GameDisplay,
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
