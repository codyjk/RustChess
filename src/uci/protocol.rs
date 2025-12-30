//! UCI protocol state machine and command execution

use std::io::{self, Write};
use std::str::FromStr;

use common::bitboard::Square;

use crate::board::Board;
use crate::chess_move::ChessMove;
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
                movetime: _,
                infinite: _,
            } => {
                self.state = UciState::Searching;
                let result = self.search_best_move(depth);
                self.state = UciState::Ready;

                match result {
                    Ok(best_move) => {
                        let uci_move = best_move.to_uci();
                        Some(UciResponseFormatter::format_bestmove_response(&uci_move))
                    }
                    Err(e) => Some(UciResponseFormatter::format_error(&e)),
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

        // TODO: Handle promotion (5th character)
        // For now, just apply the move by squares
        self.engine
            .make_move_by_squares(from_square, to_square)
            .map_err(|e| format!("Invalid move: {:?}", e))?;

        // Toggle turn after successful move
        self.engine.board_mut().toggle_turn();

        Ok(())
    }

    /// Search for the best move with optional depth override
    fn search_best_move(&mut self, depth_override: Option<u8>) -> Result<ChessMove, String> {
        // Override search depth if specified
        if let Some(depth) = depth_override {
            // For now, we'd need to modify engine depth
            // This is a limitation of current Engine API
            // For now, just use the engine's configured depth
            let _ = depth; // Suppress unused warning
        }

        self.engine
            .get_best_move()
            .map_err(|e| format!("Search failed: {:?}", e))
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

    #[test]
    fn test_go_command() {
        let mut protocol = UciProtocol::new();
        protocol.execute_command(UciCommand::Uci);
        protocol.execute_command(UciCommand::Position {
            fen: None,
            moves: vec![],
        });

        let response = protocol.execute_command(UciCommand::Go {
            depth: Some(4),
            movetime: None,
            infinite: false,
        });

        assert!(response.is_some());
        let response_str = response.unwrap();
        assert!(response_str.starts_with("bestmove "));
    }
}
