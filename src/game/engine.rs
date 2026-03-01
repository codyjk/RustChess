use std::sync::atomic::Ordering;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};

use crate::alpha_beta_searcher::{SearchContext, SearchError};
use crate::board::color::Color;
use crate::board::error::BoardError;
use crate::board::piece::Piece;
use crate::board::Board;
use crate::book::{Book, BookMove};
use crate::chess_move::algebraic_notation::enumerate_candidate_moves_with_algebraic_notation;
use crate::chess_move::chess_move::ChessMove;
use crate::chess_search::search_best_move_with_history;
use crate::evaluate::{self, GameEnding};
use crate::input_handler::MoveInput;
use crate::move_generator::MoveGenerator;
use common::bitboard::Square;
use thiserror::Error;

/// Contempt factor magnitude in centipawns. The engine scores draws as slightly
/// worse than neutral to prefer playing on. The sign is flipped based on the
/// engine's color: White (maximizer) gets -CONTEMPT_VALUE, Black (minimizer)
/// gets +CONTEMPT_VALUE, so draws are always unattractive to the searching side.
const CONTEMPT_VALUE: i16 = 25;

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
    position_hashes: Vec<u64>,
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
        let initial_hash = starting_position.current_position_hash();
        Self {
            board: starting_position,
            move_history: Vec::new(),
            position_hashes: vec![initial_hash],
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
        evaluate::game_ending(
            &mut self.state.board,
            &self.move_generator,
            turn,
            &self.state.position_hashes,
        )
    }

    pub fn position_hashes(&self) -> &[u64] {
        &self.state.position_hashes
    }

    pub fn make_move_by_squares(
        &mut self,
        from: Square,
        to: Square,
    ) -> Result<ChessMove, EngineError> {
        self.make_move_by_squares_with_promotion(from, to, None)
    }

    pub fn make_move_by_squares_with_promotion(
        &mut self,
        from: Square,
        to: Square,
        promotion: Option<Piece>,
    ) -> Result<ChessMove, EngineError> {
        let valid_moves_with_notation = self.get_valid_moves();

        let (chess_move, notation) = valid_moves_with_notation
            .iter()
            .find(|(m, _)| {
                m.from_square() == from
                    && m.to_square() == to
                    && match (promotion, m) {
                        (Some(piece), ChessMove::PawnPromotion(pm)) => {
                            pm.promote_to_piece() == piece
                        }
                        (Some(_), _) => false,
                        (None, _) => true,
                    }
            })
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

    pub fn make_best_move_with_time_limit(
        &mut self,
        time_limit: Duration,
    ) -> Result<ChessMove, EngineError> {
        let best_move = self.get_best_move_with_time_limit(time_limit)?;

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

    /// Record the current position hash. Call after turn toggle to capture
    /// the complete position state (including side to move).
    pub fn record_position_hash(&mut self) {
        self.state
            .position_hashes
            .push(self.state.board.current_position_hash());
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

    pub fn get_best_move_with_time_limit(
        &mut self,
        time_limit: Duration,
    ) -> Result<ChessMove, EngineError> {
        // Save and restore depth -- time-limited search should be bounded by time,
        // not by an artificially low depth cap.
        let saved_depth = self.search_context.search_depth();
        let max_time_depth = 100;
        if saved_depth < max_time_depth {
            self.search_context.set_depth(max_time_depth);
        }
        self.search_context.set_time_limit(Some(time_limit));

        // Check opening book first (consistent with get_best_move)
        if let Some(chess_move) = self.get_book_move() {
            self.search_context.set_time_limit(None);
            self.search_context.set_depth(saved_depth);
            return Ok(chess_move);
        }

        let result = self.run_search();

        self.search_context.set_time_limit(None);
        self.search_context.set_depth(saved_depth);
        result
    }

    pub fn set_search_depth(&mut self, depth: u8) {
        self.search_context.set_depth(depth);
    }

    pub fn search_depth(&self) -> u8 {
        self.search_context.search_depth()
    }

    fn contempt(&self) -> i16 {
        if self.state.board.turn().maximize_score() {
            -CONTEMPT_VALUE // White searching: draws score slightly negative (bad for White)
        } else {
            CONTEMPT_VALUE // Black searching: draws score slightly positive (bad for Black)
        }
    }

    /// Core search without Ctrl-C polling. Used by time-limited and UCI search paths.
    fn run_search(&mut self) -> Result<ChessMove, EngineError> {
        self.search_context.clear_stop();
        let contempt = self.contempt();
        let move_result = search_best_move_with_history(
            &mut self.search_context,
            &mut self.state.board,
            self.state.position_hashes.clone(),
            contempt,
        );
        let best_move = move_result.map_err(|err| EngineError::SearchError { error: err })?;
        self.state.last_score = self.search_context.last_score();
        Ok(best_move)
    }

    fn get_best_move_from_search(&mut self) -> Result<ChessMove, EngineError> {
        self.search_context.clear_stop();
        let stop_flag = self.search_context.stop_flag();

        // Spawn a background thread that polls for Ctrl-C during the search
        let poll_flag = stop_flag.clone();
        let poll_thread = std::thread::spawn(move || {
            while !poll_flag.load(Ordering::Relaxed) {
                if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                    if let Ok(Event::Key(key_event)) = event::read() {
                        if key_event.code == KeyCode::Char('c')
                            && key_event.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            poll_flag.store(true, Ordering::Relaxed);
                            return;
                        }
                    }
                }
            }
        });

        let contempt = self.contempt();
        let move_result = search_best_move_with_history(
            &mut self.search_context,
            &mut self.state.board,
            self.state.position_hashes.clone(),
            contempt,
        );

        // Check if user requested stop before we overwrite the flag for the polling thread
        let was_stopped = self.search_context.should_stop();

        // Signal polling thread to exit and wait for it
        stop_flag.store(true, Ordering::Relaxed);
        let _ = poll_thread.join();
        self.search_context.clear_stop();

        // If user pressed Ctrl-C, propagate Stopped even if the search returned a move
        if was_stopped {
            return Err(EngineError::SearchError {
                error: SearchError::Stopped,
            });
        }

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

    #[test]
    fn test_get_best_move_with_time_limit_returns_valid_move() {
        let mut engine = Engine::new();
        let result = engine.get_best_move_with_time_limit(Duration::from_secs(5));
        assert!(
            result.is_ok(),
            "Time-limited search should return a valid move"
        );
    }

    #[test]
    fn test_get_best_move_with_time_limit_respects_budget() {
        let mut engine = Engine::with_config(EngineConfig {
            search_depth: 20,
            starting_position: Board::default(),
        });
        let start = std::time::Instant::now();
        let _ = engine.get_best_move_with_time_limit(Duration::from_millis(100));
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_millis(500),
            "100ms budget search took {:?}, expected < 500ms",
            elapsed
        );
    }

    #[test]
    fn test_get_best_move_with_time_limit_finds_obvious_mate() {
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
            search_depth: 10,
            starting_position,
        });

        let result = engine.get_best_move_with_time_limit(Duration::from_secs(2));
        assert!(result.is_ok(), "Should find mate-in-1 with time limit");
        let chess_move = result.unwrap();
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

    #[test]
    fn test_set_search_depth_changes_depth() {
        let mut engine = Engine::new();
        assert_eq!(engine.get_search_stats().depth, 4);
        engine.set_search_depth(8);
        assert_eq!(engine.get_search_stats().depth, 8);
    }

    #[test]
    fn test_get_best_move_unchanged() {
        let mut engine = Engine::new();
        let result = engine.get_best_move();
        assert!(result.is_ok(), "Default get_best_move should still work");
    }

    #[test]
    fn test_apply_chess_move_updates_history() {
        let mut engine = Engine::new();
        let valid_moves = engine.get_valid_moves();
        let (chess_move, _) = valid_moves.first().unwrap().clone();
        engine.apply_chess_move(chess_move).unwrap();
        assert_eq!(engine.move_history().len(), 1);
    }

    #[test]
    fn test_get_valid_moves_returns_all_legal_moves() {
        let mut engine = Engine::new();
        let moves = engine.get_valid_moves();
        assert_eq!(
            moves.len(),
            20,
            "Starting position should have 20 legal moves"
        );
    }

    #[test]
    fn test_check_game_over_none_at_start() {
        let mut engine = Engine::new();
        assert!(
            engine.check_game_over().is_none(),
            "Starting position should not be game over"
        );
    }

    #[test]
    fn test_check_game_over_detects_checkmate() {
        // Black king on A1, white queen on B2, white king on C3 -- black is checkmated
        let mut position = chess_position! {
            ........
            ........
            ........
            ........
            ........
            ..K.....
            .Q......
            k.......
        };
        position.set_turn(Color::Black);
        position.lose_castle_rights(CastleRights::all());

        let mut engine = Engine::with_config(EngineConfig {
            search_depth: 4,
            starting_position: position,
        });

        let game_over = engine.check_game_over();
        assert!(
            matches!(game_over, Some(crate::evaluate::GameEnding::Checkmate)),
            "Should detect checkmate, got {:?}",
            game_over
        );
    }

    #[test]
    fn test_make_move_by_squares_with_promotion() {
        // White pawn on a7 can promote to queen on a8
        let mut position = chess_position! {
            ....k...
            P.......
            ........
            ........
            ........
            ........
            ........
            ....K...
        };
        position.set_turn(Color::White);
        position.lose_castle_rights(CastleRights::all());

        let mut engine = Engine::with_config(EngineConfig {
            search_depth: 1,
            starting_position: position,
        });

        // Promote to queen
        let result = engine.make_move_by_squares_with_promotion(A7, A8, Some(Piece::Queen));
        assert!(result.is_ok(), "Should succeed with queen promotion");
        match result.unwrap() {
            ChessMove::PawnPromotion(_) => {}
            other => panic!("Expected PawnPromotion, got {:?}", other),
        }
    }

    #[test]
    fn test_make_move_by_squares_with_promotion_knight() {
        let mut position = chess_position! {
            ....k...
            P.......
            ........
            ........
            ........
            ........
            ........
            ....K...
        };
        position.set_turn(Color::White);
        position.lose_castle_rights(CastleRights::all());

        let mut engine = Engine::with_config(EngineConfig {
            search_depth: 1,
            starting_position: position,
        });

        // Promote to knight
        let result = engine.make_move_by_squares_with_promotion(A7, A8, Some(Piece::Knight));
        assert!(result.is_ok(), "Should succeed with knight promotion");
    }

    #[test]
    fn test_threefold_repetition_detected() {
        // Simple position where we can repeat moves
        let mut position = chess_position! {
            ....k...
            ........
            ........
            ........
            ........
            ........
            ........
            ....K...
        };
        position.set_turn(Color::White);
        position.lose_castle_rights(CastleRights::all());

        let mut engine = Engine::with_config(EngineConfig {
            search_depth: 1,
            starting_position: position,
        });

        // Move king back and forth: Ke1-d1, Ke8-d8, Kd1-e1, Kd8-e8 (back to start = 2nd
        // occurrence), Ke1-d1, Ke8-d8, Kd1-e1, Kd8-e8 (back to start = 3rd occurrence)
        let moves = [
            (E1, D1),
            (E8, D8),
            (D1, E1),
            (D8, E8), // 2nd occurrence of start
            (E1, D1),
            (E8, D8),
            (D1, E1),
            (D8, E8), // 3rd occurrence of start
        ];
        for (from, to) in &moves {
            engine.make_move_by_squares(*from, *to).unwrap();
            engine.board_mut().toggle_turn();
            engine.record_position_hash();
        }

        let game_over = engine.check_game_over();
        assert!(
            matches!(game_over, Some(crate::evaluate::GameEnding::Draw)),
            "Should detect threefold repetition as draw, got {:?}",
            game_over
        );
    }

    #[test]
    fn test_position_hashes_tracked() {
        let mut engine = Engine::new();
        // Starting position hash is already tracked
        assert_eq!(
            engine.position_hashes().len(),
            1,
            "Should have initial position hash"
        );

        // Make a move
        engine.make_move_by_squares(E2, E4).unwrap();
        engine.board_mut().toggle_turn();
        engine.record_position_hash();
        assert_eq!(
            engine.position_hashes().len(),
            2,
            "Should have 2 hashes after one move"
        );
    }

    #[test]
    fn test_make_best_move_with_time_limit() {
        let mut engine = Engine::new();
        let result = engine.make_best_move_with_time_limit(Duration::from_secs(1));
        assert!(result.is_ok(), "Should make a move with time limit");
        assert_eq!(
            engine.move_history().len(),
            1,
            "Should have one move in history"
        );
    }
}
