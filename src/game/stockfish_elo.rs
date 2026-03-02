use std::io;
use std::time::{Duration, Instant};

use common::bitboard::*;
use crossterm::{cursor, execute, terminal};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

use crate::board::color::Color;
use crate::board::piece::Piece;
use crate::board::Board;
use crate::evaluate::GameEnding;
use crate::game::engine::{Engine, EngineConfig};
use crate::game::stockfish_interface::Stockfish;
use crate::tui::{board_widget::BoardWidget, Theme};

const GAMES_PER_ELO: usize = 10;
const ELO_INCREMENT: u32 = 25;

pub fn determine_stockfish_elo(depth: u8, starting_elo: u32, no_tui: bool, time_limit_ms: u64) {
    let mut stockfish = match Stockfish::new() {
        Ok(sf) => sf,
        Err(_) => {
            eprintln!("Error: Stockfish not found. Please ensure it's installed and in your PATH.");
            return;
        }
    };

    let mut renderer: Box<dyn EloRenderer> = if no_tui {
        Box::new(HeadlessRenderer)
    } else {
        match EloTui::new() {
            Ok(tui) => Box::new(tui),
            Err(e) => {
                eprintln!("Failed to initialize TUI: {}", e);
                return;
            }
        }
    };

    let min_elo = stockfish.min_elo();
    let mut current_elo = starting_elo.max(min_elo);
    let mut engine_total_time = Duration::new(0, 0);
    let mut stockfish_total_time = Duration::new(0, 0);

    loop {
        stockfish.set_elo(current_elo).unwrap();

        let mut level_wins = 0;
        let mut level_losses = 0;
        let mut level_draws = 0;
        let mut level_games = 0;

        for _ in 0..GAMES_PER_ELO {
            let stats = EloStats {
                current_elo,
                wins: level_wins,
                losses: level_losses,
                draws: level_draws,
                total_games: level_games,
                engine_total_time,
                stockfish_total_time,
            };

            let (result, engine_time, sf_time) = play_game(
                &mut stockfish,
                depth,
                time_limit_ms,
                renderer.as_mut(),
                &stats,
            );
            level_games += 1;
            engine_total_time += engine_time;
            stockfish_total_time += sf_time;

            match result {
                GameResult::Win => level_wins += 1,
                GameResult::Loss => level_losses += 1,
                GameResult::Draw => level_draws += 1,
            }

            if is_elo_determined(level_wins, level_draws, level_games) {
                let final_stats = EloStats {
                    current_elo,
                    wins: level_wins,
                    losses: level_losses,
                    draws: level_draws,
                    total_games: level_games,
                    engine_total_time,
                    stockfish_total_time,
                };
                renderer.render_final(&final_stats).ok();
                return;
            }
        }

        // Use chess score rate: wins count 1, draws count 0.5
        let score = level_wins as f32 + level_draws as f32 * 0.5;
        if score > level_games as f32 * 0.5 {
            current_elo += ELO_INCREMENT;
        } else if current_elo <= min_elo {
            // Can't go below Stockfish's minimum -- report this as the floor
            let final_stats = EloStats {
                current_elo: min_elo,
                wins: level_wins,
                losses: level_losses,
                draws: level_draws,
                total_games: level_games,
                engine_total_time,
                stockfish_total_time,
            };
            renderer.render_final(&final_stats).ok();
            return;
        } else {
            current_elo = current_elo.saturating_sub(ELO_INCREMENT).max(min_elo);
        }
    }
}

fn play_game(
    stockfish: &mut Stockfish,
    depth: u8,
    time_limit_ms: u64,
    renderer: &mut dyn EloRenderer,
    stats: &EloStats,
) -> (GameResult, Duration, Duration) {
    let mut engine = Engine::with_config(EngineConfig {
        search_depth: depth,
        starting_position: Board::default(),
    });
    let mut moves = Vec::new();
    let mut engine_time = Duration::new(0, 0);
    let mut stockfish_time = Duration::new(0, 0);

    let engine_color = Color::random();

    loop {
        if let Some(result) = engine.check_game_over() {
            return (
                match result {
                    GameEnding::Checkmate => {
                        if engine.board().turn() == engine_color {
                            GameResult::Loss
                        } else {
                            GameResult::Win
                        }
                    }
                    _ => GameResult::Draw,
                },
                engine_time,
                stockfish_time,
            );
        }

        let start_time = Instant::now();
        let current_turn = engine.board().turn();

        if current_turn == engine_color {
            match engine.make_best_move_with_time_limit(Duration::from_millis(time_limit_ms)) {
                Ok(chess_move) => {
                    engine_time += start_time.elapsed();
                    moves.push(chess_move.to_uci());
                }
                Err(_) => return (GameResult::Draw, engine_time, stockfish_time),
            }
        } else {
            let (sf_move, sf_time) = match stockfish.get_best_move(&moves.join(" "), time_limit_ms)
            {
                Ok(result) => result,
                Err(_) => return (GameResult::Draw, engine_time, stockfish_time),
            };
            stockfish_time += Duration::from_millis(sf_time);

            let from = match Square::from_algebraic(&sf_move[0..2]) {
                Some(sq) => sq,
                None => return (GameResult::Draw, engine_time, stockfish_time),
            };
            let to = match Square::from_algebraic(&sf_move[2..4]) {
                Some(sq) => sq,
                None => return (GameResult::Draw, engine_time, stockfish_time),
            };
            let promotion = sf_move.chars().nth(4).map(|c| match c {
                'q' => Piece::Queen,
                'r' => Piece::Rook,
                'b' => Piece::Bishop,
                'n' => Piece::Knight,
                _ => Piece::Queen,
            });

            match engine.make_move_by_squares_with_promotion(from, to, promotion) {
                Ok(chess_move) => moves.push(chess_move.to_uci()),
                Err(e) => {
                    let valid_moves = engine.get_valid_moves();
                    let valid_uci: Vec<String> =
                        valid_moves.iter().map(|(m, _)| m.to_uci()).collect();
                    eprintln!(
                        "Board desync with Stockfish: {}\n  \
                         FEN: {}\n  \
                         SF move: {}\n  \
                         Turn: {:?}\n  \
                         Move list: {}\n  \
                         Valid moves: {:?}",
                        e,
                        engine.board().to_fen(),
                        sf_move,
                        engine.board().turn(),
                        moves.join(" "),
                        valid_uci
                    );
                    return (GameResult::Draw, engine_time, stockfish_time);
                }
            }
        }

        // Render the current game state
        let game_state = GameState {
            engine_color,
            engine_time,
            stockfish_time,
        };
        renderer.render(&engine, &game_state, stats).ok();

        engine.board_mut().toggle_turn();
        engine.record_position_hash();
    }
}

/// Check if ELO is determined using chess score rate (wins=1, draws=0.5).
fn is_elo_determined(wins: usize, draws: usize, total_games: usize) -> bool {
    if total_games < GAMES_PER_ELO {
        return false;
    }
    let score_rate = (wins as f32 + draws as f32 * 0.5) / total_games as f32;
    (score_rate - 0.5).abs() < 0.1
}

enum GameResult {
    Win,
    Loss,
    Draw,
}

/// Statistics for ELO determination progress
struct EloStats {
    current_elo: u32,
    wins: usize,
    losses: usize,
    draws: usize,
    total_games: usize,
    engine_total_time: Duration,
    stockfish_total_time: Duration,
}

/// Game state for rendering
struct GameState {
    engine_color: Color,
    engine_time: Duration,
    stockfish_time: Duration,
}

/// Rendering abstraction for ELO determination progress
trait EloRenderer {
    fn render(
        &mut self,
        engine: &Engine,
        game_state: &GameState,
        elo_stats: &EloStats,
    ) -> io::Result<()>;

    fn render_final(&mut self, elo_stats: &EloStats) -> io::Result<()>;
}

/// Headless renderer that prints progress to stdout
struct HeadlessRenderer;

impl EloRenderer for HeadlessRenderer {
    fn render(
        &mut self,
        engine: &Engine,
        game_state: &GameState,
        elo_stats: &EloStats,
    ) -> io::Result<()> {
        // Print a summary line after each game completes (when move count resets).
        // We detect "new game" by checking if the board is near starting position.
        let move_count = engine.move_history().len();
        if move_count <= 1 && elo_stats.total_games > 0 {
            let score_rate = (elo_stats.wins as f32 + elo_stats.draws as f32 * 0.5)
                / elo_stats.total_games as f32
                * 100.0;
            println!(
                "  ELO {} | Game {} | W/L/D: {}/{}/{} | Score: {:.1}% | \
                 Engine: {:.1}s | SF: {:.1}s",
                elo_stats.current_elo,
                elo_stats.total_games,
                elo_stats.wins,
                elo_stats.losses,
                elo_stats.draws,
                score_rate,
                game_state.engine_time.as_secs_f64(),
                game_state.stockfish_time.as_secs_f64(),
            );
        }
        Ok(())
    }

    fn render_final(&mut self, elo_stats: &EloStats) -> io::Result<()> {
        let score_rate = if elo_stats.total_games > 0 {
            (elo_stats.wins as f32 + elo_stats.draws as f32 * 0.5) / elo_stats.total_games as f32
                * 100.0
        } else {
            0.0
        };
        println!("\nELO DETERMINATION COMPLETE");
        println!("Final ELO: {}", elo_stats.current_elo);
        println!(
            "Games: {} | W/L/D: {}/{}/{} | Score: {:.1}%",
            elo_stats.total_games, elo_stats.wins, elo_stats.losses, elo_stats.draws, score_rate
        );
        Ok(())
    }
}

/// TUI for ELO determination
struct EloTui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    theme: Theme,
}

impl EloTui {
    fn new() -> io::Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            terminal,
            theme: Theme::default(),
        })
    }

    fn render_game_info(
        frame: &mut ratatui::Frame,
        area: Rect,
        engine: &Engine,
        game_state: &GameState,
        theme: &Theme,
    ) {
        let mut info_text = String::new();

        // FEN
        info_text.push_str(&format!("FEN: {}\n\n", engine.board().to_fen()));

        // Turn
        info_text.push_str(&format!("Turn: {}\n\n", engine.board().turn()));

        // Colors
        info_text.push_str(&format!("Engine: {}\n", game_state.engine_color));
        info_text.push_str(&format!(
            "Stockfish: {}\n\n",
            game_state.engine_color.opposite()
        ));

        // Engine stats
        let stats = engine.get_search_stats();
        info_text.push_str("Engine Stats:\n");
        info_text.push_str(&format!("  Depth: {}\n", stats.depth));

        if stats.positions_searched > 0 {
            info_text.push_str(&format!(
                "  Nodes: {}\n",
                format_number(stats.positions_searched as u64)
            ));
        } else {
            info_text.push_str("  Nodes: -\n");
        }

        if let Some(duration) = stats.last_search_duration {
            info_text.push_str(&format!("  Time: {:.2}s\n", duration.as_secs_f64()));
        } else {
            info_text.push_str("  Time: -\n");
        }

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

    fn render_elo_stats(
        frame: &mut ratatui::Frame,
        area: Rect,
        game_state: &GameState,
        elo_stats: &EloStats,
        theme: &Theme,
    ) {
        let score_rate = if elo_stats.total_games > 0 {
            (elo_stats.wins as f32 + elo_stats.draws as f32 * 0.5) / elo_stats.total_games as f32
                * 100.0
        } else {
            0.0
        };

        let avg_engine_time = if elo_stats.total_games > 0 {
            elo_stats.engine_total_time.as_millis() as f32 / elo_stats.total_games as f32
        } else {
            0.0
        };

        let avg_stockfish_time = if elo_stats.total_games > 0 {
            elo_stats.stockfish_total_time.as_millis() as f32 / elo_stats.total_games as f32
        } else {
            0.0
        };

        let stats_text = format!(
            "Target ELO: {}  │  Games: {}  │  W/L/D: {}/{}/{}  │  Score: {:.1}%\n\
             Engine avg: {:.0}ms  │  Stockfish avg: {:.0}ms\n\
             \n\
             Current game timings:\n\
             Engine: {:.0}ms  │  Stockfish: {:.0}ms",
            elo_stats.current_elo,
            elo_stats.total_games,
            elo_stats.wins,
            elo_stats.losses,
            elo_stats.draws,
            score_rate,
            avg_engine_time,
            avg_stockfish_time,
            game_state.engine_time.as_millis(),
            game_state.stockfish_time.as_millis()
        );

        let paragraph = Paragraph::new(stats_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("ELO Determination Progress"),
            )
            .style(theme.text_style());

        frame.render_widget(paragraph, area);
    }
}

impl EloRenderer for EloTui {
    fn render(
        &mut self,
        engine: &Engine,
        game_state: &GameState,
        elo_stats: &EloStats,
    ) -> io::Result<()> {
        self.terminal.clear()?;

        let theme = &self.theme;
        self.terminal.draw(|f| {
            let size = f.area();

            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(10), Constraint::Length(10)])
                .split(size);

            let board_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(main_chunks[0]);

            let board_widget = BoardWidget::new(engine.board(), theme);
            f.render_widget(board_widget, board_chunks[0]);

            Self::render_game_info(f, board_chunks[1], engine, game_state, theme);
            Self::render_elo_stats(f, main_chunks[1], game_state, elo_stats, theme);
        })?;

        Ok(())
    }

    fn render_final(&mut self, elo_stats: &EloStats) -> io::Result<()> {
        self.terminal.clear()?;

        let theme = &self.theme;
        self.terminal.draw(|f| {
            let size = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(size);

            let score_rate = (elo_stats.wins as f32 + elo_stats.draws as f32 * 0.5)
                / elo_stats.total_games as f32
                * 100.0;
            let final_text = format!(
                "ELO DETERMINATION COMPLETE\n\
                 \n\
                 Final ELO: {}\n\
                 \n\
                 Total Games: {}\n\
                 Wins: {}\n\
                 Losses: {}\n\
                 Draws: {}\n\
                 Score: {:.1}%\n\
                 \n\
                 Press any key to exit...",
                elo_stats.current_elo,
                elo_stats.total_games,
                elo_stats.wins,
                elo_stats.losses,
                elo_stats.draws,
                score_rate
            );

            let paragraph = Paragraph::new(final_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("ELO Determination Results"),
                )
                .style(theme.text_style());

            f.render_widget(paragraph, chunks[0]);
        })?;

        // Wait for key press
        loop {
            if crossterm::event::poll(Duration::from_millis(100))? {
                if let crossterm::event::Event::Key(_) = crossterm::event::read()? {
                    break;
                }
            }
        }

        Ok(())
    }
}

impl Drop for EloTui {
    fn drop(&mut self) {
        let _ = execute!(
            io::stdout(),
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0)
        );
        let _ = crossterm::terminal::disable_raw_mode();
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- is_elo_determined tests (uses score rate: wins=1, draws=0.5) ---

    #[test]
    fn test_is_elo_determined_50_percent_score_rate() {
        // 5 wins, 0 draws, 5 losses -> score = 5/10 = 50%
        assert!(
            is_elo_determined(5, 0, 10),
            "50% score rate at 10 games should be determined"
        );
    }

    #[test]
    fn test_is_elo_determined_draws_count_half() {
        // 4 wins, 2 draws, 4 losses -> score = (4+1)/10 = 50%
        assert!(
            is_elo_determined(4, 2, 10),
            "4W/2D/4L = 50% score rate should be determined"
        );
    }

    #[test]
    fn test_is_elo_determined_high_score_rate() {
        // 8 wins, 0 draws -> score = 8/10 = 80%
        assert!(
            !is_elo_determined(8, 0, 10),
            "80% score rate should not be determined"
        );
    }

    #[test]
    fn test_is_elo_determined_low_score_rate() {
        // 2 wins, 0 draws -> score = 2/10 = 20%
        assert!(
            !is_elo_determined(2, 0, 10),
            "20% score rate should not be determined"
        );
    }

    #[test]
    fn test_is_elo_determined_insufficient_games() {
        assert!(
            !is_elo_determined(3, 0, 5),
            "Only 5 games should not be determined"
        );
    }

    #[test]
    fn test_is_elo_determined_many_draws_is_determined() {
        // 3 wins, 4 draws, 3 losses -> score = (3+2)/10 = 50%
        assert!(
            is_elo_determined(3, 4, 10),
            "3W/4D/3L = 50% score rate should be determined"
        );
    }
}
