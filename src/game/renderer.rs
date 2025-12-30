use crate::board::color::Color;
use crate::chess_move::chess_move::ChessMove;
use crate::game::display::GameDisplay;
use crate::game::engine::Engine;
use std::time::Duration;

pub trait GameRenderer {
    fn render(
        &self,
        ui: &mut GameDisplay,
        engine: &Engine,
        current_turn: Color,
        last_move: Option<(&ChessMove, &str)>,
    );
    fn frame_delay(&self) -> Option<Duration>;
}

pub struct SimpleRenderer;

impl GameRenderer for SimpleRenderer {
    fn render(
        &self,
        ui: &mut GameDisplay,
        engine: &Engine,
        current_turn: Color,
        last_move: Option<(&ChessMove, &str)>,
    ) {
        let opening_name = engine.get_book_line_name();
        ui.render_game_state(
            engine.board(),
            current_turn,
            last_move,
            None,
            opening_name.as_deref(),
        );
        println!("Enter your move:");
    }

    fn frame_delay(&self) -> Option<Duration> {
        None
    }
}

pub struct StatsRenderer {
    pub delay_between_moves: Option<Duration>,
}

impl GameRenderer for StatsRenderer {
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
        let opening_name = engine.get_book_line_name();
        ui.render_game_state(
            engine.board(),
            current_turn,
            last_move,
            Some(&stats_display),
            opening_name.as_deref(),
        );
    }

    fn frame_delay(&self) -> Option<Duration> {
        self.delay_between_moves
    }
}

pub struct ConditionalStatsRenderer {
    pub human_color: Color,
}

impl GameRenderer for ConditionalStatsRenderer {
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
        let opening_name = engine.get_book_line_name();
        ui.render_game_state(
            engine.board(),
            current_turn,
            last_move,
            Some(&stats_display),
            opening_name.as_deref(),
        );
        if current_turn == self.human_color {
            println!("Enter your move:");
        }
    }

    fn frame_delay(&self) -> Option<Duration> {
        None
    }
}
