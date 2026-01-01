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
use crate::chess_move::capture::Capture;
use crate::chess_move::castle::CastleChessMove;
use crate::chess_move::chess_move::ChessMove;
use crate::chess_move::en_passant::EnPassantChessMove;
use crate::chess_move::pawn_promotion::PawnPromotionChessMove;
use crate::chess_move::standard::StandardChessMove;
use crate::evaluate::GameEnding;
use crate::game::engine::{Engine, EngineConfig};
use crate::game::stockfish_interface::Stockfish;
use crate::tui::{board_widget::BoardWidget, Theme};

const GAMES_PER_ELO: usize = 10;
const ELO_INCREMENT: u32 = 25;
const TIME_LIMIT: u64 = 1000; // 1 second per move

pub fn determine_stockfish_elo(depth: u8, starting_elo: u32) {
    let mut stockfish = match Stockfish::new() {
        Ok(sf) => sf,
        Err(_) => {
            eprintln!("Error: Stockfish not found. Please ensure it's installed and in your PATH.");
            return;
        }
    };

    // Initialize TUI
    let mut tui = match EloTui::new() {
        Ok(tui) => tui,
        Err(e) => {
            eprintln!("Failed to initialize TUI: {}", e);
            return;
        }
    };

    let mut current_elo = starting_elo;
    let mut wins = 0;
    let mut losses = 0;
    let mut draws = 0;
    let mut total_games = 0;
    let mut engine_total_time = Duration::new(0, 0);
    let mut stockfish_total_time = Duration::new(0, 0);

    loop {
        stockfish.set_elo(current_elo).unwrap();

        for _ in 0..GAMES_PER_ELO {
            let stats = EloStats {
                current_elo,
                wins,
                losses,
                draws,
                total_games,
                engine_total_time,
                stockfish_total_time,
            };

            let (result, engine_time, sf_time) = play_game(&mut stockfish, depth, &mut tui, &stats);
            total_games += 1;
            engine_total_time += engine_time;
            stockfish_total_time += sf_time;

            match result {
                GameResult::Win => wins += 1,
                GameResult::Loss => losses += 1,
                GameResult::Draw => draws += 1,
            }

            if is_elo_determined(wins, losses, total_games) {
                let final_stats = EloStats {
                    current_elo,
                    wins,
                    losses,
                    draws,
                    total_games,
                    engine_total_time,
                    stockfish_total_time,
                };
                tui.render_final(&final_stats).ok();
                return;
            }
        }

        if wins > losses {
            current_elo += ELO_INCREMENT;
        } else {
            current_elo -= ELO_INCREMENT;
        }

        wins = 0;
        losses = 0;
        draws = 0;
    }
}

fn play_game(
    stockfish: &mut Stockfish,
    depth: u8,
    tui: &mut EloTui,
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

        let chess_move = if current_turn == engine_color {
            let chess_move = engine.get_best_move().unwrap();
            engine_time += start_time.elapsed();
            chess_move
        } else {
            let (sf_move, sf_time) = stockfish
                .get_best_move(&moves.join(" "), TIME_LIMIT)
                .unwrap();
            stockfish_time += Duration::from_millis(sf_time);
            create_chess_move_from_uci(&sf_move, engine.board())
        };

        engine.apply_chess_move(chess_move.clone()).unwrap();
        moves.push(chess_move.to_uci());

        // Render the current game state
        let game_state = GameState {
            engine_color,
            engine_time,
            stockfish_time,
        };
        tui.render(&engine, &game_state, stats).ok();

        engine.board_mut().toggle_turn();
    }
}

fn create_chess_move_from_uci(uci: &str, board: &Board) -> ChessMove {
    let from = Square::from_algebraic(&uci[0..2]).expect("Invalid from square");
    let to = Square::from_algebraic(&uci[2..4]).expect("Invalid to square");
    let promotion = uci.chars().nth(4).map(|c| match c {
        'q' => Piece::Queen,
        'r' => Piece::Rook,
        'b' => Piece::Bishop,
        'n' => Piece::Knight,
        _ => panic!("Invalid promotion piece"),
    });

    let piece = board
        .get(from)
        .expect("from square should contain a piece")
        .0;
    let capture = board.get(to).map(|(p, _)| Capture(p));

    match (piece, promotion) {
        (Piece::Pawn, Some(promote_to)) => {
            ChessMove::PawnPromotion(PawnPromotionChessMove::new(from, to, capture, promote_to))
        }
        (Piece::Pawn, None) if Some(to) == board.peek_en_passant_target() => {
            ChessMove::EnPassant(EnPassantChessMove::new(from, to))
        }
        (Piece::King, None) if (from, to) == (E1, G1) || (from, to) == (E8, G8) => {
            ChessMove::Castle(CastleChessMove::castle_kingside(board.turn()))
        }
        (Piece::King, None) if (from, to) == (E1, C1) || (from, to) == (E8, C8) => {
            ChessMove::Castle(CastleChessMove::castle_queenside(board.turn()))
        }
        _ => ChessMove::Standard(StandardChessMove::new(from, to, capture)),
    }
}

fn is_elo_determined(wins: usize, _losses: usize, total_games: usize) -> bool {
    total_games >= GAMES_PER_ELO && (wins as f32 / total_games as f32 - 0.5).abs() < 0.1
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

            // Main layout: board area + status panel at bottom
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(10), Constraint::Length(10)])
                .split(size);

            // Split board area: board on left, info panel on right
            let board_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(main_chunks[0]);

            // Render board
            let board_widget = BoardWidget::new(engine.board(), theme);
            f.render_widget(board_widget, board_chunks[0]);

            // Render game info panel
            Self::render_game_info(f, board_chunks[1], engine, game_state, theme);

            // Render ELO stats panel
            Self::render_elo_stats(f, main_chunks[1], game_state, elo_stats, theme);
        })?;

        Ok(())
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
        let win_rate = if elo_stats.total_games > 0 {
            elo_stats.wins as f32 / elo_stats.total_games as f32 * 100.0
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
            "Target ELO: {}  │  Games: {}  │  W/L/D: {}/{}/{}  │  Win Rate: {:.1}%\n\
             Engine avg: {:.0}ms  │  Stockfish avg: {:.0}ms\n\
             \n\
             Current game timings:\n\
             Engine: {:.0}ms  │  Stockfish: {:.0}ms",
            elo_stats.current_elo,
            elo_stats.total_games,
            elo_stats.wins,
            elo_stats.losses,
            elo_stats.draws,
            win_rate,
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

    fn render_final(&mut self, elo_stats: &EloStats) -> io::Result<()> {
        self.terminal.clear()?;

        let theme = &self.theme;
        self.terminal.draw(|f| {
            let size = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(size);

            let final_text = format!(
                "ELO DETERMINATION COMPLETE\n\
                 \n\
                 Final ELO: {}\n\
                 \n\
                 Total Games: {}\n\
                 Wins: {}\n\
                 Losses: {}\n\
                 Draws: {}\n\
                 Win Rate: {:.1}%\n\
                 \n\
                 Press any key to exit...",
                elo_stats.current_elo,
                elo_stats.total_games,
                elo_stats.wins,
                elo_stats.losses,
                elo_stats.draws,
                (elo_stats.wins as f32 / elo_stats.total_games as f32 * 100.0)
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
