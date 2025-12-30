//! Main TUI application state and rendering

use std::io;

use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

use crate::board::color::Color;
use crate::chess_move::ChessMove;
use crate::game::engine::Engine;
use crate::tui::{board_widget::BoardWidget, Theme};

/// Format large numbers with thousand separators
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// Main TUI application
pub struct TuiApp {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    theme: Theme,
    should_quit: bool,
}

impl TuiApp {
    /// Create a new TUI application
    pub fn new() -> io::Result<Self> {
        // Setup terminal without alternate screen for compatibility with stdin input
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            terminal,
            theme: Theme::default(),
            should_quit: false,
        })
    }

    /// Run the TUI application
    pub fn run(
        &mut self,
        engine: &Engine,
        current_turn: Color,
        last_move: Option<(&ChessMove, &str)>,
        opening_name: Option<&str>,
        human_color: Option<Color>,
    ) -> io::Result<()> {
        // Clear terminal backend state to force full redraw after manual screen clear
        self.terminal.clear()?;

        let theme = &self.theme;
        self.terminal.draw(|f| {
            Self::render_frame(
                f,
                engine,
                current_turn,
                last_move,
                opening_name,
                theme,
                human_color,
            );
        })?;

        // Position cursor in the input box when it's a human's turn
        let should_show_cursor = match human_color {
            None => true, // PvP - always show cursor
            Some(color) => current_turn == color,
        };

        if should_show_cursor {
            let height = self.terminal.size()?.height;
            // Input panel is at height - 3, cursor goes after prompt text
            print!("\x1B[{};19H", height - 1); // Row: height-1, Column: 19 (after prompt)
            use std::io::Write;
            std::io::stdout().flush()?;
        }

        Ok(())
    }

    /// Render a single frame
    fn render_frame(
        frame: &mut ratatui::Frame,
        engine: &Engine,
        current_turn: Color,
        last_move: Option<(&ChessMove, &str)>,
        opening_name: Option<&str>,
        theme: &Theme,
        human_color: Option<Color>,
    ) {
        let size = frame.area();

        // Create main layout: board area + input panel at bottom
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(10), Constraint::Length(3)])
            .split(size);

        // Split board area: board on left, info panel on right
        let board_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(main_chunks[0]);

        // Render board
        let board_widget = BoardWidget::new(engine.board(), theme);
        frame.render_widget(board_widget, board_chunks[0]);

        // Render info panel
        Self::render_info_panel(
            frame,
            board_chunks[1],
            engine,
            current_turn,
            last_move,
            opening_name,
            theme,
        );

        // Render input panel at bottom
        Self::render_input_panel(frame, main_chunks[1], current_turn, human_color, theme);
    }

    /// Render the info panel with game details
    fn render_info_panel(
        frame: &mut ratatui::Frame,
        area: Rect,
        engine: &Engine,
        current_turn: Color,
        last_move: Option<(&ChessMove, &str)>,
        opening_name: Option<&str>,
        theme: &Theme,
    ) {
        let mut info_text = String::new();

        // Opening name
        if let Some(opening) = opening_name {
            info_text.push_str(&format!("Opening: {}\n\n", opening));
        }

        // Current turn
        info_text.push_str(&format!("Turn: {}\n\n", current_turn));

        // Last move
        if let Some((_mv, notation)) = last_move {
            info_text.push_str(&format!("Last Move: {}\n\n", notation));
        }

        // Engine stats
        let stats = engine.get_search_stats();
        info_text.push_str("Engine Stats:\n");
        info_text.push_str(&format!("  Depth: {}\n", stats.depth));

        // Format large numbers with separators
        if stats.positions_searched > 0 {
            info_text.push_str(&format!(
                "  Nodes: {}\n",
                format_number(stats.positions_searched as u64)
            ));
        } else {
            info_text.push_str("  Nodes: -\n");
        }

        // Show time or placeholder
        if let Some(duration) = stats.last_search_duration {
            info_text.push_str(&format!("  Time: {:.2}s\n", duration.as_secs_f64()));
        } else {
            info_text.push_str("  Time: -\n");
        }

        // Show score or placeholder
        if let Some(score) = stats.last_score {
            info_text.push_str(&format!("  Score: {}\n", score));
        } else {
            info_text.push_str("  Score: -\n");
        }

        let paragraph = Paragraph::new(info_text)
            .block(Block::default().borders(Borders::ALL).title("Game Info"))
            .style(theme.text_style());

        frame.render_widget(paragraph, area);
    }

    /// Render the input panel at the bottom
    fn render_input_panel(
        frame: &mut ratatui::Frame,
        area: Rect,
        current_turn: Color,
        human_color: Option<Color>,
        theme: &Theme,
    ) {
        let prompt_text = match human_color {
            None => "Enter your move: _", // PvP mode - both sides are human
            Some(color) if current_turn == color => "Enter your move: _",
            Some(_) => "Engine is thinking...",
        };

        let paragraph = Paragraph::new(prompt_text)
            .block(Block::default().borders(Borders::ALL).title("Input"))
            .style(theme.text_style());

        frame.render_widget(paragraph, area);
    }

    /// Check if should quit
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// Wait for user input (blocking)
    pub fn wait_for_key() -> io::Result<()> {
        loop {
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        _ => return Ok(()), // Any other key continues
                    }
                }
            }
        }
    }
}

impl Drop for TuiApp {
    fn drop(&mut self) {
        // Cleanup is minimal since we don't use alternate screen or raw mode
    }
}
