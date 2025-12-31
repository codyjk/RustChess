use std::time::Duration;

use crate::alpha_beta_searcher::{SearchContext, SearchError};
use crate::board::color::Color;
use crate::board::error::BoardError;
use crate::board::Board;
use crate::book::{Book, BookMove};
use crate::chess_move::algebraic_notation::enumerate_candidate_moves_with_algebraic_notation;
use crate::chess_move::chess_move::ChessMove;
use crate::chess_search::search_best_move;
use crate::evaluate::{self, GameEnding};
use crate::input_handler::MoveInput;
use crate::move_generator::MoveGenerator;
use common::bitboard::Square;
use thiserror::Error;

/// Core engine state and configuration
#[derive(Clone)]
pub struct EngineConfig {
    pub search_depth: u8,
    pub starting_position: Board,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            search_depth: 4, // Default search depth
            starting_position: Board::default(),
        }
    }
}

/// Represents a move in the game history with its notation and score
#[derive(Clone, Debug)]
pub struct MoveHistoryEntry {
    pub chess_move: ChessMove,
    pub notation: String,
    pub score: Option<i16>,
}

/// Game state and runtime info
#[derive(Clone)]
pub struct GameState {
    board: Board,
    move_history: Vec<MoveHistoryEntry>,
    last_score: Option<i16>,
    opening_deviation_move: Option<usize>,
    last_known_opening: Option<String>,
}

impl Default for GameState {
    fn default() -> Self {
        Self::new(Board::default())
    }
}

impl GameState {
    fn new(starting_position: Board) -> Self {
        Self {
            board: starting_position,
            move_history: Vec::new(),
            last_score: None,
            opening_deviation_move: None,
            last_known_opening: None,
        }
    }
}

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Invalid move")]
    InvalidMove,
    #[error("Board error: {error:?}")]
    BoardError { error: BoardError },
    #[error("Search error: {error:?}")]
    SearchError { error: SearchError },
}

/// The main chess engine that manages game state and provides move generation/analysis
pub struct Engine {
    state: GameState,
    book: Book,
    move_generator: MoveGenerator,
    search_context: SearchContext<ChessMove>,
}

impl Default for Engine {
    fn default() -> Self {
        Self::with_config(EngineConfig::default())
    }
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(config: EngineConfig) -> Self {
        Self {
            state: GameState::new(config.starting_position),
            book: Book::default(),
            move_generator: MoveGenerator::default(),
            search_context: SearchContext::new(config.search_depth),
        }
    }

    pub fn board(&self) -> &Board {
        &self.state.board
    }

    pub fn board_mut(&mut self) -> &mut Board {
        &mut self.state.board
    }

    pub fn get_valid_moves(&mut self) -> Vec<(ChessMove, String)> {
        let board = &mut self.state.board;
        let current_turn = board.turn();
        enumerate_candidate_moves_with_algebraic_notation(board, current_turn, &self.move_generator)
    }

    pub fn check_game_over(&mut self) -> Option<GameEnding> {
        let turn = self.state.board.turn();
        evaluate::game_ending(&mut self.state.board, &self.move_generator, turn)
    }

    pub fn make_move_by_squares(
        &mut self,
        from: Square,
        to: Square,
    ) -> Result<ChessMove, EngineError> {
        let valid_moves_with_notation = self.get_valid_moves();

        let (chess_move, notation) = valid_moves_with_notation
            .iter()
            .find(|(m, _)| m.from_square() == from && m.to_square() == to)
            .ok_or(EngineError::InvalidMove)?
            .clone();

        self.apply_chess_move_with_notation(chess_move.clone(), notation, self.state.last_score)?;
        Ok(chess_move)
    }

    pub fn make_move_algebraic(&mut self, algebraic: String) -> Result<ChessMove, EngineError> {
        let valid_moves = self.get_valid_moves();
        let (chess_move, notation) = valid_moves
            .iter()
            .find(|(_, n)| n == &algebraic)
            .ok_or(EngineError::InvalidMove)?
            .clone();

        self.apply_chess_move_with_notation(chess_move.clone(), notation, self.state.last_score)?;
        Ok(chess_move)
    }

    pub fn get_best_move(&mut self) -> Result<ChessMove, EngineError> {
        // Try opening book first
        let book_move = self.get_book_move();
        if let Some(chess_move) = book_move {
            return Ok(chess_move);
        }

        // Fall back to search
        self.get_best_move_from_search()
    }

    pub fn make_best_move(&mut self) -> Result<ChessMove, EngineError> {
        let best_move = self.get_best_move()?;

        // Get notation for the move
        // Match by squares (from/to) instead of full equality to handle cases where
        // check/checkmate effects aren't set on the best_move yet
        let valid_moves = self.get_valid_moves();
        let notation = valid_moves
            .iter()
            .find(|(m, _)| {
                m.from_square() == best_move.from_square() && m.to_square() == best_move.to_square()
            })
            .map(|(_, n)| n.clone())
            .expect("best_move should always be in valid_moves");

        self.apply_chess_move_with_notation(best_move.clone(), notation, self.state.last_score)?;
        Ok(best_move)
    }

    pub fn get_score(&mut self, current_turn: Color) -> i16 {
        evaluate::score(&mut self.state.board, &self.move_generator, current_turn, 0)
    }

    pub fn get_search_stats(&self) -> SearchStats {
        SearchStats {
            positions_searched: self.search_context.searched_position_count(),
            depth: self.search_context.search_depth(),
            last_score: self.state.last_score,
            last_search_duration: self.search_context.last_search_duration(),
        }
    }

    pub fn get_book_line_name(&self) -> Option<String> {
        let line = self.get_book_line();
        let current_opening = self.book.get_line(line);

        // Return current opening if available, otherwise return last known opening
        current_opening.or_else(|| self.state.last_known_opening.clone())
    }

    pub fn last_move(&self) -> Option<ChessMove> {
        self.state.move_history.last().map(|e| e.chess_move.clone())
    }

    pub fn move_history(&self) -> &[MoveHistoryEntry] {
        &self.state.move_history
    }

    pub fn opening_deviation_move(&self) -> Option<usize> {
        self.state.opening_deviation_move
    }

    /// Apply a chess move without tracking notation or score (for internal use)
    pub fn apply_chess_move(&mut self, chess_move: ChessMove) -> Result<(), EngineError> {
        chess_move
            .apply(&mut self.state.board)
            .map_err(|error| EngineError::BoardError { error })?;

        // For moves applied without notation, we still need to add to history
        // Use UCI notation (e.g., "e2e4") for compact display
        let notation = chess_move.to_uci();
        self.state.move_history.push(MoveHistoryEntry {
            chess_move,
            notation,
            score: None,
        });

        Ok(())
    }

    pub fn apply_chess_move_with_notation(
        &mut self,
        chess_move: ChessMove,
        notation: String,
        score: Option<i16>,
    ) -> Result<(), EngineError> {
        // Check if this move deviates from the opening book
        if self.state.opening_deviation_move.is_none() {
            // Save the current opening name before checking deviation
            let current_line = self.get_book_line();
            if let Some(opening_name) = self.book.get_line(current_line) {
                self.state.last_known_opening = Some(opening_name);
            }

            let next_moves = self.book.get_next_moves(self.get_book_line());
            let book_move =
                crate::book::BookMove::new(chess_move.from_square(), chess_move.to_square());
            let is_in_book = next_moves.iter().any(|(mv, _)| *mv == book_move);

            if !is_in_book {
                self.state.opening_deviation_move = Some(self.state.move_history.len() + 1);
            }
        }

        chess_move
            .apply(&mut self.state.board)
            .map_err(|error| EngineError::BoardError { error })?;

        self.state.move_history.push(MoveHistoryEntry {
            chess_move,
            notation,
            score,
        });

        Ok(())
    }

    pub fn make_move_from_input(&mut self, input: MoveInput) -> Result<ChessMove, EngineError> {
        match input {
            MoveInput::Coordinate { from, to } => {
                let from_square = Square::from_algebraic(&from).ok_or(EngineError::InvalidMove)?;
                let to_square = Square::from_algebraic(&to).ok_or(EngineError::InvalidMove)?;
                self.make_move_by_squares(from_square, to_square)
            }
            MoveInput::Algebraic { notation } => self.make_move_algebraic(notation),
            MoveInput::UseEngine => self.make_best_move(),
        }
    }

    // Private helper methods

    fn get_book_move(&mut self) -> Option<ChessMove> {
        let current_turn = self.state.board.turn();
        let line = self.get_book_line();
        let candidate_moves = self.book.get_next_moves(line);

        if candidate_moves.is_empty() {
            return None;
        }

        // Pick random book move
        let (book_move, _) = &candidate_moves[fastrand::usize(..candidate_moves.len())];
        let from_square = book_move.from_square();
        let to_square = book_move.to_square();

        let candidates = self
            .move_generator
            .generate_moves(&mut self.state.board, current_turn);

        candidates
            .into_iter()
            .find(|m| m.from_square() == from_square && m.to_square() == to_square)
    }

    fn get_best_move_from_search(&mut self) -> Result<ChessMove, EngineError> {
        let move_result = search_best_move(&mut self.search_context, &mut self.state.board);

        let best_move = move_result.map_err(|err| EngineError::SearchError { error: err })?;
        self.state.last_score = self.search_context.last_score();

        Ok(best_move)
    }

    fn get_book_line(&self) -> Vec<BookMove> {
        self.state
            .move_history
            .iter()
            .map(|entry| {
                BookMove::new(entry.chess_move.from_square(), entry.chess_move.to_square())
            })
            .collect()
    }
}

/// Search performance statistics
#[derive(Debug, Clone)]
pub struct SearchStats {
    pub positions_searched: usize,
    pub depth: u8,
    pub last_score: Option<i16>,
    pub last_search_duration: Option<Duration>,
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::castle_rights::CastleRights;
    use crate::board::piece::Piece;
    use crate::chess_move::chess_move_effect::ChessMoveEffect;
    use crate::chess_move::standard::StandardChessMove;
    use crate::chess_position;
    use crate::{checkmate_move, std_move};
    use common::bitboard::*;

    #[test]
    fn test_find_mate_in_1_white() {
        let mut starting_position = chess_position! {
            .Q......
            ........
            ........
            ........
            ........
            ........
            k.K.....
            ........
        };
        starting_position.set_turn(Color::White);
        starting_position.lose_castle_rights(CastleRights::all());

        let mut engine = Engine::with_config(EngineConfig {
            search_depth: 4,
            starting_position,
        });

        let chess_move = engine.get_best_move().unwrap();
        let valid_checkmates = [
            checkmate_move!(std_move!(B8, B2)),
            checkmate_move!(std_move!(B8, A8)),
            checkmate_move!(std_move!(B8, A7)),
        ];
        assert!(
            valid_checkmates.contains(&chess_move),
            "{} does not lead to checkmate",
            chess_move
        );
    }
}
