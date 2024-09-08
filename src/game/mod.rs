pub mod command;
pub mod modes;

use crate::alpha_beta_searcher::{alpha_beta_search, SearchContext, SearchError};
use crate::board::color::Color;
use crate::board::error::BoardError;
use crate::board::Board;
use crate::book::Book;
use crate::chess_move::algebraic_notation::enumerate_candidate_moves_with_algebraic_notation;
use crate::chess_move::ChessMove;
use crate::evaluate::{self, GameEnding};
use crate::move_generator::MoveGenerator;
use common::bitboard::bitboard::Bitboard;
use rand::{self, Rng};
use thiserror::Error;

/// Represents the state and control of a chess game.
pub struct Game {
    board: Board,
    move_history: Vec<ChessMove>,
    book: Book,
    move_generator: MoveGenerator,
    search_context: SearchContext,
}

#[derive(Error, Debug)]
pub enum GameError {
    #[error("that is not a valid move")]
    InvalidMove,
    #[error("board error: {error:?}")]
    BoardError { error: BoardError },
    #[error("search error: {error:?}")]
    SearchError { error: SearchError },
}

impl Game {
    pub fn new(search_depth: u8) -> Self {
        Self::from_board(Board::starting_position(), search_depth)
    }

    pub fn from_board(board: Board, search_depth: u8) -> Self {
        Self {
            board,
            move_history: Vec::new(),
            book: Book::new(),
            move_generator: MoveGenerator::new(),
            search_context: SearchContext::new(search_depth),
        }
    }

    pub fn check_game_over_for_current_turn(&mut self) -> Option<GameEnding> {
        let turn = self.board.turn();
        evaluate::game_ending(&mut self.board, &mut self.move_generator, turn)
    }

    pub fn save_move(&mut self, chess_move: ChessMove) {
        self.move_history.push(chess_move);
    }

    pub fn most_recent_move(&self) -> Option<ChessMove> {
        self.move_history.iter().last().cloned()
    }

    pub fn apply_chess_move_by_from_to_square(
        &mut self,
        from_square: Bitboard,
        to_square: Bitboard,
    ) -> Result<ChessMove, GameError> {
        let turn = self.board.turn();
        let candidates = self.move_generator.generate_moves(&mut self.board, turn);
        let chess_move = candidates
            .iter()
            .find(|m| m.from_square() == from_square && m.to_square() == to_square)
            .ok_or(GameError::InvalidMove)?;
        self.apply_chess_move(chess_move.clone())?;
        Ok(chess_move.clone())
    }

    pub fn apply_chess_move(&mut self, chess_move: ChessMove) -> Result<(), GameError> {
        match chess_move.apply(&mut self.board) {
            Ok(_capture) => {
                self.save_move(chess_move.clone());
                Ok(())
            }
            Err(error) => Err(GameError::BoardError { error }),
        }
    }

    pub fn apply_chess_move_from_raw_algebraic_notation(
        &mut self,
        algebraic: String,
    ) -> Result<ChessMove, GameError> {
        let board = &mut self.board;
        let current_turn = board.turn();
        let move_generator = &mut self.move_generator;
        let enumerated_candidate_moves =
            enumerate_candidate_moves_with_algebraic_notation(board, current_turn, move_generator);
        let chess_move = enumerated_candidate_moves
            .iter()
            .find(|m| m.1 == algebraic)
            .ok_or(GameError::InvalidMove)?
            .0
            .clone();
        self.apply_chess_move(chess_move.clone())?;
        Ok(chess_move)
    }

    pub fn make_alpha_beta_best_move(&mut self) -> Result<ChessMove, GameError> {
        let best_move = match alpha_beta_search(
            &mut self.search_context,
            &mut self.board,
            &mut self.move_generator,
        ) {
            Ok(mv) => mv,
            Err(err) => return Err(GameError::SearchError { error: err }),
        };

        match best_move.apply(&mut self.board) {
            Ok(_capture) => {
                self.save_move(best_move.clone());
                Ok(best_move)
            }
            Err(error) => Err(GameError::BoardError { error }),
        }
    }

    pub fn make_waterfall_book_then_alpha_beta_move(&mut self) -> Result<ChessMove, GameError> {
        let current_turn = self.board.turn();
        let line = self
            .move_history
            .iter()
            .map(|m| (m.from_square(), m.to_square()))
            .collect();
        let book_moves = self.book.get_next_moves(line);

        if book_moves.is_empty() {
            return self.make_alpha_beta_best_move();
        }

        let rng = rand::thread_rng().gen_range(0..book_moves.len());
        let (from_square, to_square) = book_moves[rng];
        let candidates = self
            .move_generator
            .generate_moves(&mut self.board, current_turn);

        let maybe_chess_move = candidates
            .iter()
            .find(|m| m.from_square() == from_square && m.to_square() == to_square);

        let chess_move = match maybe_chess_move {
            Some(result) => result,
            None => return Err(GameError::InvalidMove),
        };

        match chess_move.apply(&mut self.board) {
            Ok(_capture) => {
                self.save_move(chess_move.clone());
                Ok(chess_move.clone())
            }
            Err(error) => Err(GameError::BoardError { error }),
        }
    }

    pub fn score(&mut self, current_turn: Color) -> i16 {
        evaluate::score(&mut self.board, &mut self.move_generator, current_turn)
    }

    pub fn fullmove_clock(&self) -> u8 {
        self.board.fullmove_clock()
    }

    pub fn searched_position_count(&self) -> usize {
        self.search_context.searched_position_count()
    }

    pub fn alpha_beta_cache_hit_count(&self) -> usize {
        self.search_context.cache_hit_count()
    }

    pub fn alpha_beta_termination_count(&self) -> usize {
        self.search_context.termination_count()
    }

    pub fn search_depth(&self) -> u8 {
        self.search_context.search_depth()
    }

    pub fn move_generator_cache_hit_count(&self) -> usize {
        self.move_generator.cache_hit_count()
    }

    pub fn move_genereator_cache_entry_count(&self) -> usize {
        self.move_generator.cache_entry_count()
    }

    pub fn reset_move_generator_cache_hit_count(&mut self) {
        self.move_generator.reset_cache_hit_count();
    }

    pub fn last_move(&self) -> Option<ChessMove> {
        self.move_history.last().cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::castle_rights_bitmask::ALL_CASTLE_RIGHTS;
    use crate::board::piece::Piece;
    use crate::chess_move::standard::StandardChessMove;
    use crate::chess_move::ChessMove;
    use crate::{chess_position, std_move};
    use common::bitboard::square;

    #[test]
    fn test_score() {
        let mut game = Game::new(0);
        game.apply_chess_move_by_from_to_square(square::E2, square::E4)
            .unwrap();
        game.board.toggle_turn();
        assert!(game.check_game_over_for_current_turn().is_none());
    }

    #[test]
    fn test_checkmate() {
        let mut game = Game::new(0);
        game.apply_chess_move_by_from_to_square(square::F2, square::F3)
            .unwrap();
        game.board.toggle_turn();
        game.apply_chess_move_by_from_to_square(square::E7, square::E6)
            .unwrap();
        game.board.toggle_turn();
        game.apply_chess_move_by_from_to_square(square::G2, square::G4)
            .unwrap();
        game.board.toggle_turn();
        game.apply_chess_move_by_from_to_square(square::D8, square::H4)
            .unwrap();
        game.board.toggle_turn();
        println!("Testing board:\n{}", game.board);
        matches!(
            game.check_game_over_for_current_turn(),
            Some(GameEnding::Checkmate)
        );
    }

    #[test]
    fn test_draw_from_repetition() {
        let mut board = chess_position! {
            .......k
            .......r
            ........
            ........
            ........
            ........
            K.......
            R.......
        };
        board.set_turn(Color::White);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        // make sure starting position has been counted
        board.count_current_position();

        let mut game = Game::from_board(board, 0);
        println!("Testing board:\n{}", game.board);

        let first_moves = [
            std_move!(square::A2, square::A3),
            std_move!(square::H8, square::G8),
            std_move!(square::A3, square::A2),
            std_move!(square::G8, square::H8),
        ];

        for m in first_moves.iter() {
            m.apply(&mut game.board).unwrap();
            game.board.toggle_turn();
        }

        // back in starting position for second time
        matches!(
            game.check_game_over_for_current_turn(),
            Some(GameEnding::Draw)
        );

        let second_moves = [
            std_move!(square::A2, square::A3),
            std_move!(square::H8, square::G8),
            std_move!(square::A3, square::A2),
            std_move!(square::G8, square::H8),
        ];

        for m in second_moves.iter() {
            m.apply(&mut game.board).unwrap();
            game.board.toggle_turn();
        }

        // back in starting position for third time, should be draw
        matches!(
            game.check_game_over_for_current_turn(),
            Some(GameEnding::Draw)
        );
    }
}
