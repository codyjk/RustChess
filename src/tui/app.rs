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
use crate::evaluate::GameEnding;
use crate::game::engine::Engine;
use crate::tui::{board_widget::BoardWidget, Theme};

/// Game state information for rendering
struct GameState<'a> {
    current_turn: Color,
    last_move: Option<(&'a ChessMove, &'a str)>,
    opening_name: Option<&'a str>,
    human_color: Option<Color>,
    game_ending: Option<&'a GameEnding>,
}

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
        game_ending: Option<&GameEnding>,
    ) -> io::Result<()> {
        // Clear terminal backend state to force full redraw after manual screen clear
        self.terminal.clear()?;

        let theme = &self.theme;
        let game_state = GameState {
            current_turn,
            last_move,
            opening_name,
            human_color,
            game_ending,
        };
        self.terminal.draw(|f| {
            Self::render_frame(f, engine, &game_state, theme);
        })?;

        // Position cursor in the input box when it's a human's turn and game hasn't ended
        let should_show_cursor = game_state.game_ending.is_none()
            && match game_state.human_color {
                None => true, // PvP - always show cursor
                Some(color) => game_state.current_turn == color,
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
        game_state: &GameState,
        theme: &Theme,
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
        Self::render_info_panel(frame, board_chunks[1], engine, game_state, theme);

        // Render input panel at bottom
        Self::render_input_panel(frame, main_chunks[1], game_state, theme);
    }

    /// Render the info panel with game details
    fn render_info_panel(
        frame: &mut ratatui::Frame,
        area: Rect,
        engine: &Engine,
        game_state: &GameState,
        theme: &Theme,
    ) {
        let mut info_text = String::new();
        let is_watch_mode = game_state.human_color.is_none();

        // Opening name with deviation info
        if let Some(opening) = game_state.opening_name {
            info_text.push_str(&format!("Opening: {}", opening));
            if let Some(deviation_move) = engine.opening_deviation_move() {
                info_text.push_str(&format!(" (ended on move {})", deviation_move));
            }
            info_text.push_str("\n\n");
        }

        // FEN
        info_text.push_str(&format!("FEN: {}\n\n", engine.board().to_fen()));

        // Current turn
        info_text.push_str(&format!("Turn: {}\n\n", game_state.current_turn));

        // Last move
        if let Some((_mv, notation)) = game_state.last_move {
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
            info_text.push_str(&format!("  Score: {}\n\n", score));
        } else {
            info_text.push_str("  Score: -\n\n");
        }

        // Move history table (at bottom so it grows downward)
        let move_history = engine.move_history();
        if !move_history.is_empty() {
            info_text.push_str("Move History:\n");

            // Table header
            if is_watch_mode {
                info_text.push_str("  # │ White      │ Black      │ Score\n");
                info_text.push_str("  ──┼────────────┼────────────┼────────\n");
            } else {
                info_text.push_str("  # │ White      │ Black\n");
                info_text.push_str("  ──┼────────────┼────────────\n");
            }

            // Process moves in pairs
            for i in (0..move_history.len()).step_by(2) {
                let move_number = (i / 2) + 1;
                let white_move = &move_history[i];
                let black_move = move_history.get(i + 1);

                if is_watch_mode {
                    let white_score = white_move
                        .score
                        .map(|s| format!("{:>6}", s))
                        .unwrap_or_else(|| "     -".to_string());
                    let black_score = black_move
                        .and_then(|m| m.score)
                        .map(|s| format!("{:>6}", s))
                        .unwrap_or_else(|| "     -".to_string());

                    if let Some(black) = black_move {
                        info_text.push_str(&format!(
                            " {:>2} │ {:<10} │ {:<10} │ {}/{}\n",
                            move_number,
                            white_move.notation,
                            black.notation,
                            white_score,
                            black_score
                        ));
                    } else {
                        info_text.push_str(&format!(
                            " {:>2} │ {:<10} │            │ {}\n",
                            move_number, white_move.notation, white_score
                        ));
                    }
                } else if let Some(black) = black_move {
                    info_text.push_str(&format!(
                        " {:>2} │ {:<10} │ {:<10}\n",
                        move_number, white_move.notation, black.notation
                    ));
                } else {
                    info_text.push_str(&format!(
                        " {:>2} │ {:<10} │\n",
                        move_number, white_move.notation
                    ));
                }
            }
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
        game_state: &GameState,
        theme: &Theme,
    ) {
        let prompt_text = if let Some(ending) = game_state.game_ending {
            match ending {
                GameEnding::Checkmate => "Checkmate!",
                GameEnding::Stalemate => "Stalemate!",
                GameEnding::Draw => "Draw!",
            }
        } else {
            match game_state.human_color {
                None => "Watch mode - engines playing...", // Watch mode - both sides are engine
                Some(color) if game_state.current_turn == color => "Enter your move: _",
                Some(_) => "Engine is thinking...",
            }
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
