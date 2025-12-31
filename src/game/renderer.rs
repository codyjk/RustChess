use std::cell::RefCell;
use std::io;
use std::time::Duration;

use crate::board::color::Color;
use crate::chess_move::chess_move::ChessMove;
use crate::evaluate::GameEnding;
use crate::game::display::GameDisplay;
use crate::game::engine::Engine;
use crate::tui::TuiApp;

pub trait GameRenderer {
    fn render(
        &self,
        ui: &mut GameDisplay,
        engine: &Engine,
        current_turn: Color,
        last_move: Option<(&ChessMove, &str)>,
        game_ending: Option<&GameEnding>,
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
        game_ending: Option<&GameEnding>,
    ) {
        let opening_name = engine.get_book_line_name();
        ui.render_game_state(
            engine.board(),
            current_turn,
            last_move,
            None,
            opening_name.as_deref(),
        );
        if let Some(ending) = game_ending {
            match ending {
                GameEnding::Checkmate => println!("Checkmate!"),
                GameEnding::Stalemate => println!("Stalemate!"),
                GameEnding::Draw => println!("Draw!"),
            }
        } else {
            println!("Enter your move:");
        }
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
        game_ending: Option<&GameEnding>,
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
        if let Some(ending) = game_ending {
            match ending {
                GameEnding::Checkmate => println!("Checkmate!"),
                GameEnding::Stalemate => println!("Stalemate!"),
                GameEnding::Draw => println!("Draw!"),
            }
        }
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
        game_ending: Option<&GameEnding>,
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
        if let Some(ending) = game_ending {
            match ending {
                GameEnding::Checkmate => println!("Checkmate!"),
                GameEnding::Stalemate => println!("Stalemate!"),
                GameEnding::Draw => println!("Draw!"),
            }
        } else if current_turn == self.human_color {
            println!("Enter your move:");
        }
    }

    fn frame_delay(&self) -> Option<Duration> {
        None
    }
}

pub struct TuiRenderer {
    app: RefCell<TuiApp>,
    human_color: Option<Color>, // None means both players are human (pvp mode)
}

impl TuiRenderer {
    pub fn new(human_color: Option<Color>) -> io::Result<Self> {
        Ok(Self {
            app: RefCell::new(TuiApp::new()?),
            human_color,
        })
    }
}

impl GameRenderer for TuiRenderer {
    fn render(
        &self,
        _ui: &mut GameDisplay,
        engine: &Engine,
        current_turn: Color,
        last_move: Option<(&ChessMove, &str)>,
        game_ending: Option<&GameEnding>,
    ) {
        // Clear screen once at the start of each render for clean display
        print!("\x1B[2J\x1B[1;1H");

        let opening_name = engine.get_book_line_name();
        let _ = self.app.borrow_mut().run(
            engine,
            current_turn,
            last_move,
            opening_name.as_deref(),
            self.human_color,
            game_ending,
        );

        // Input prompt is now integrated into the TUI layout
    }

    fn frame_delay(&self) -> Option<Duration> {
        None
    }
}
