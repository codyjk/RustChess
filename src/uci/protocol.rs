//! UCI protocol state machine and command execution

use std::io::{self, Write};
use std::str::FromStr;

use common::bitboard::Square;

use crate::board::piece::Piece;
use crate::board::Board;
use crate::game::engine::{Engine, EngineConfig};

use super::command_parser::UciCommand;
use super::response_formatter::UciResponseFormatter;

/// Current state of the UCI protocol
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum UciState {
    /// Waiting for 'uci' command to initialize
    WaitingForUci,
    /// Ready to receive commands
    Ready,
    /// Currently searching
    Searching,
}

/// Calculate time allocation for a single move.
///
/// Uses `time_remaining / 30 + increment * 80%`, clamped to not exceed
/// `time_remaining - 50ms` safety margin.
fn allocate_time(time_remaining_ms: u64, increment_ms: u64) -> u64 {
    let base = time_remaining_ms / 30;
    let inc_bonus = increment_ms * 4 / 5;
    let budget = base + inc_bonus;
    let safety_margin = 50;
    let max_allowed = time_remaining_ms.saturating_sub(safety_margin);
    budget.min(max_allowed).max(1) // always at least 1ms
}

/// UCI protocol handler
pub struct UciProtocol {
    state: UciState,
    engine: Engine,
    should_quit: bool,
}

impl Default for UciProtocol {
    fn default() -> Self {
        Self::new()
    }
}

impl UciProtocol {
    /// Create a new UCI protocol handler
    pub fn new() -> Self {
        Self {
            state: UciState::WaitingForUci,
            engine: Engine::with_config(EngineConfig::default()),
            should_quit: false,
        }
    }

    /// Check if the protocol should quit
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// Execute a UCI command and return the response
    pub fn execute_command(&mut self, command: UciCommand) -> Option<String> {
        match command {
            UciCommand::Uci => {
                self.state = UciState::Ready;
                Some(UciResponseFormatter::format_uci_response())
            }

            UciCommand::IsReady => Some(UciResponseFormatter::format_ready_response()),

            UciCommand::Position { fen, moves } => {
                if let Err(e) = self.set_position(fen, moves) {
                    Some(UciResponseFormatter::format_error(&e))
                } else {
                    None
                }
            }

            UciCommand::Go {
                depth,
                movetime,
                wtime,
                btime,
                winc,
                binc,
                infinite,
            } => {
                self.state = UciState::Searching;

                // Determine time budget and depth
                let time_budget_ms = if let Some(mt) = movetime {
                    // movetime takes precedence
                    Some(mt)
                } else if !infinite {
                    // Calculate time budget from clock
                    let turn = self.engine.board().turn();
                    let (time_for_side, increment) = match turn {
                        crate::board::color::Color::White => {
                            (wtime.unwrap_or(0), winc.unwrap_or(0))
                        }
                        crate::board::color::Color::Black => {
                            (btime.unwrap_or(0), binc.unwrap_or(0))
                        }
                    };
                    if time_for_side > 0 {
                        Some(allocate_time(time_for_side, increment))
                    } else {
                        None
                    }
                } else {
                    None // infinite
                };

                // Reset depth to default before each search -- depth is per-command,
                // not persistent across go commands.
                const DEFAULT_DEPTH: u8 = 4;
                self.engine.set_search_depth(depth.unwrap_or(DEFAULT_DEPTH));

                let result = if let Some(budget_ms) = time_budget_ms {
                    let budget = std::time::Duration::from_millis(budget_ms);
                    self.engine.get_best_move_with_time_limit(budget)
                } else {
                    self.engine.get_best_move()
                };

                self.state = UciState::Ready;

                match result {
                    Ok(best_move) => {
                        let uci_move = best_move.to_uci();
                        Some(UciResponseFormatter::format_bestmove_response(&uci_move))
                    }
                    Err(e) => Some(UciResponseFormatter::format_error(&format!("{:?}", e))),
                }
            }

            UciCommand::Stop => {
                // For now, we don't support stopping mid-search
                // Since our search is synchronous
                self.state = UciState::Ready;
                None
            }

            UciCommand::Quit => {
                self.should_quit = true;
                None
            }

            UciCommand::SetOption { name: _, value: _ } => {
                // Options not yet implemented
                None
            }

            UciCommand::Unknown(cmd) => {
                if !cmd.is_empty() {
                    Some(UciResponseFormatter::format_error(&format!(
                        "Unknown command: {}",
                        cmd
                    )))
                } else {
                    None
                }
            }
        }
    }

    /// Set the board position from FEN or startpos, optionally applying moves
    fn set_position(&mut self, fen: Option<String>, moves: Vec<String>) -> Result<(), String> {
        // Create board from FEN or use starting position
        let board = if let Some(fen_string) = fen {
            Board::from_str(&fen_string).map_err(|e| format!("Invalid FEN: {:?}", e))?
        } else {
            Board::default()
        };

        // Create new engine with this position
        let config = EngineConfig {
            search_depth: 4, // Default depth, will be overridden by 'go depth N'
            starting_position: board,
        };
        self.engine = Engine::with_config(config);

        // Apply moves if any
        for move_str in moves {
            self.apply_uci_move(&move_str)?;
        }

        Ok(())
    }

    /// Apply a single UCI move to the engine
    fn apply_uci_move(&mut self, uci_move: &str) -> Result<(), String> {
        // UCI moves are in format "e2e4" or "e7e8q" (with promotion)
        if uci_move.len() < 4 || uci_move.len() > 5 {
            return Err(format!("Invalid UCI move format: {}", uci_move));
        }

        let from_square = Square::from_algebraic(&uci_move[0..2])
            .ok_or_else(|| format!("Invalid from square: {}", &uci_move[0..2]))?;
        let to_square = Square::from_algebraic(&uci_move[2..4])
            .ok_or_else(|| format!("Invalid to square: {}", &uci_move[2..4]))?;

        let promotion = uci_move.chars().nth(4).map(|c| match c {
            'q' => Piece::Queen,
            'r' => Piece::Rook,
            'b' => Piece::Bishop,
            'n' => Piece::Knight,
            _ => Piece::Queen,
        });

        self.engine
            .make_move_by_squares_with_promotion(from_square, to_square, promotion)
            .map_err(|e| format!("Invalid move: {:?}", e))?;

        // Toggle turn after successful move
        self.engine.board_mut().toggle_turn();
        self.engine.record_position_hash();

        Ok(())
    }

    /// Run the UCI protocol loop, reading from stdin and writing to stdout
    pub fn run(&mut self) {
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            // Read command from stdin
            let mut input = String::new();
            if stdin.read_line(&mut input).is_err() {
                break;
            }

            // Parse command
            let command = match input.parse::<UciCommand>() {
                Ok(cmd) => cmd,
                Err(e) => {
                    let error_response = UciResponseFormatter::format_error(&e);
                    writeln!(stdout, "{}", error_response).ok();
                    stdout.flush().ok();
                    continue;
                }
            };

            // Execute command
            if let Some(response) = self.execute_command(command) {
                writeln!(stdout, "{}", response).ok();
                stdout.flush().ok();
            }

            // Check if we should quit
            if self.should_quit() {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let protocol = UciProtocol::new();
        assert_eq!(protocol.state, UciState::WaitingForUci);
        assert!(!protocol.should_quit());
    }

    #[test]
    fn test_uci_command() {
        let mut protocol = UciProtocol::new();
        let response = protocol.execute_command(UciCommand::Uci);
        assert!(response.is_some());
        assert_eq!(protocol.state, UciState::Ready);
    }

    #[test]
    fn test_isready_command() {
        let mut protocol = UciProtocol::new();
        protocol.execute_command(UciCommand::Uci);
        let response = protocol.execute_command(UciCommand::IsReady);
        assert_eq!(response, Some("readyok".to_string()));
    }

    #[test]
    fn test_quit_command() {
        let mut protocol = UciProtocol::new();
        protocol.execute_command(UciCommand::Quit);
        assert!(protocol.should_quit());
    }

    #[test]
    fn test_position_startpos() {
        let mut protocol = UciProtocol::new();
        protocol.execute_command(UciCommand::Uci);
        let result = protocol.execute_command(UciCommand::Position {
            fen: None,
            moves: vec![],
        });
        assert!(result.is_none()); // No response expected for position command
    }

    #[test]
    fn test_position_with_moves() {
        let mut protocol = UciProtocol::new();
        protocol.execute_command(UciCommand::Uci);
        let result = protocol.execute_command(UciCommand::Position {
            fen: None,
            moves: vec!["e2e4".to_string(), "e7e5".to_string()],
        });
        assert!(result.is_none()); // No error expected
    }

    fn go_cmd(
        depth: Option<u8>,
        movetime: Option<u64>,
        wtime: Option<u64>,
        btime: Option<u64>,
        winc: Option<u64>,
        binc: Option<u64>,
        infinite: bool,
    ) -> UciCommand {
        UciCommand::Go {
            depth,
            movetime,
            wtime,
            btime,
            winc,
            binc,
            infinite,
        }
    }

    #[test]
    fn test_go_command() {
        let mut protocol = UciProtocol::new();
        protocol.execute_command(UciCommand::Uci);
        protocol.execute_command(UciCommand::Position {
            fen: None,
            moves: vec![],
        });

        let response =
            protocol.execute_command(go_cmd(Some(4), None, None, None, None, None, false));
        assert!(response.is_some());
        let response_str = response.unwrap();
        assert!(response_str.starts_with("bestmove "));
    }

    #[test]
    fn test_go_movetime_uses_time_limit() {
        let mut protocol = UciProtocol::new();
        protocol.execute_command(UciCommand::Uci);
        protocol.execute_command(UciCommand::Position {
            fen: None,
            moves: vec![],
        });

        let start = std::time::Instant::now();
        let response =
            protocol.execute_command(go_cmd(None, Some(500), None, None, None, None, false));
        let elapsed = start.elapsed();

        assert!(response.is_some());
        assert!(response.unwrap().starts_with("bestmove "));
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "movetime 500ms should finish well under 2s, took {:?}",
            elapsed
        );
    }

    #[test]
    fn test_go_wtime_btime_uses_time_budget() {
        let mut protocol = UciProtocol::new();
        protocol.execute_command(UciCommand::Uci);
        protocol.execute_command(UciCommand::Position {
            fen: None,
            moves: vec![],
        });

        let response = protocol.execute_command(go_cmd(
            None,
            None,
            Some(60000),
            Some(60000),
            None,
            None,
            false,
        ));
        assert!(response.is_some());
        assert!(response.unwrap().starts_with("bestmove "));
    }

    #[test]
    fn test_go_wtime_btime_with_increment() {
        let mut protocol = UciProtocol::new();
        protocol.execute_command(UciCommand::Uci);
        protocol.execute_command(UciCommand::Position {
            fen: None,
            moves: vec![],
        });

        let response = protocol.execute_command(go_cmd(
            None,
            None,
            Some(10000),
            Some(10000),
            Some(1000),
            Some(1000),
            false,
        ));
        assert!(response.is_some());
        assert!(response.unwrap().starts_with("bestmove "));
    }

    #[test]
    fn test_go_depth_override_works() {
        let mut protocol = UciProtocol::new();
        protocol.execute_command(UciCommand::Uci);
        protocol.execute_command(UciCommand::Position {
            fen: None,
            moves: vec![],
        });

        // Depth 1 should be very fast
        let start = std::time::Instant::now();
        let response =
            protocol.execute_command(go_cmd(Some(1), None, None, None, None, None, false));
        let elapsed_d1 = start.elapsed();

        assert!(response.is_some());
        assert!(response.unwrap().starts_with("bestmove "));
        assert!(
            elapsed_d1 < std::time::Duration::from_millis(500),
            "Depth 1 should be very fast, took {:?}",
            elapsed_d1
        );
    }

    #[test]
    fn test_time_allocation_formula() {
        // Standard game: 60s remaining, no increment
        let budget = allocate_time(60000, 0);
        assert_eq!(budget, 2000, "60s / 30 = 2s");

        // With increment: 10s remaining, 1s increment
        let budget = allocate_time(10000, 1000);
        assert_eq!(budget, 10000 / 30 + 800, "10s/30 + 1s*0.8");

        // Very low time: 100ms, no increment
        let budget = allocate_time(100, 0);
        assert!(
            budget <= 50,
            "100ms should budget at most 50ms (safety margin)"
        );
        assert!(budget >= 1, "Always at least 1ms");

        // Zero time remaining
        let budget = allocate_time(0, 0);
        assert_eq!(budget, 1, "Zero time should still give 1ms minimum");
    }

    #[test]
    fn test_position_fen_then_go() {
        let mut protocol = UciProtocol::new();
        protocol.execute_command(UciCommand::Uci);
        protocol.execute_command(UciCommand::Position {
            fen: Some("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1".to_string()),
            moves: vec![],
        });

        let response =
            protocol.execute_command(go_cmd(Some(2), None, None, None, None, None, false));
        assert!(response.is_some());
        assert!(response.unwrap().starts_with("bestmove "));
    }

    #[test]
    fn test_position_with_moves_then_go() {
        let mut protocol = UciProtocol::new();
        protocol.execute_command(UciCommand::Uci);
        protocol.execute_command(UciCommand::Position {
            fen: None,
            moves: vec!["e2e4".to_string(), "e7e5".to_string()],
        });

        let response =
            protocol.execute_command(go_cmd(Some(2), None, None, None, None, None, false));
        assert!(response.is_some());
        assert!(response.unwrap().starts_with("bestmove "));
    }

    #[test]
    fn test_go_depth_does_not_persist() {
        let mut protocol = UciProtocol::new();
        protocol.execute_command(UciCommand::Uci);
        protocol.execute_command(UciCommand::Position {
            fen: None,
            moves: vec![],
        });

        // First search at depth 1
        protocol.execute_command(go_cmd(Some(1), None, None, None, None, None, false));

        // Second search without explicit depth should use default (4), not 1.
        // Verify via the engine's configured search depth after Go resets it.
        let response = protocol.execute_command(go_cmd(None, None, None, None, None, None, false));
        assert!(response.is_some());
        assert!(response.unwrap().starts_with("bestmove "));
        // After Go with no depth, engine should be at default depth (4), not 1
        assert_eq!(
            protocol.engine.search_depth(),
            4,
            "Search depth should reset to default (4) after go without explicit depth"
        );
    }
}
