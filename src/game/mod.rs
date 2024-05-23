pub mod command;
pub mod modes;

use crate::board::color::Color;
use crate::board::error::BoardError;
use crate::board::Board;
use crate::book::{generate_opening_book, Book};
use crate::evaluate::{self, GameEnding};
use crate::moves;
use crate::moves::chess_move::ChessMove;
use crate::moves::targets::Targets;
use crate::searcher::{SearchError, Searcher};
use rand::{self, Rng};
use thiserror::Error;

pub struct Game {
    board: Board,
    move_history: Vec<ChessMove>,
    book: Book,
    targets: Targets,
    searcher: Searcher,
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
            move_history: vec![],
            book: generate_opening_book(),
            targets: Targets::new(),
            searcher: Searcher::new(search_depth),
        }
    }

    pub fn check_game_over_for_current_turn(&mut self) -> Option<GameEnding> {
        let turn = self.board.turn();
        evaluate::game_ending(&mut self.board, &mut self.targets, turn)
    }

    pub fn save_move(&mut self, chessmove: ChessMove) {
        self.move_history.push(chessmove)
    }

    pub fn make_move(&mut self, from_square: u64, to_square: u64) -> Result<ChessMove, GameError> {
        let turn = self.board.turn();
        let candidates = moves::generate(&mut self.board, turn, &mut self.targets);
        let maybe_chessmove = candidates
            .iter()
            .find(|&m| m.from_square() == from_square && m.to_square() == to_square);
        let chessmove = match maybe_chessmove {
            Some(result) => *result,
            None => return Err(GameError::InvalidMove),
        };
        match self.board.apply(chessmove) {
            Ok(_capture) => {
                self.save_move(chessmove);
                Ok(chessmove)
            }
            Err(error) => Err(GameError::BoardError { error }),
        }
    }

    pub fn make_alpha_beta_best_move(&mut self) -> Result<ChessMove, GameError> {
        let best_move = match self.searcher.search(&mut self.board, &mut self.targets) {
            Ok(mv) => mv,
            Err(err) => return Err(GameError::SearchError { error: err }),
        };

        match self.board.apply(best_move) {
            Ok(_capture) => {
                self.save_move(best_move);
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
            .map(|cm| (cm.from_square(), cm.to_square()))
            .collect();
        let book_moves = self.book.get_next_moves(line);

        if book_moves.is_empty() {
            return self.make_alpha_beta_best_move();
        }

        let rng = rand::thread_rng().gen_range(0..book_moves.len());
        let (from_square, to_square) = book_moves[rng];
        let candidates = moves::generate(&mut self.board, current_turn, &mut self.targets);

        let maybe_chessmove = candidates
            .iter()
            .find(|&m| m.from_square() == from_square && m.to_square() == to_square);

        let chessmove = match maybe_chessmove {
            Some(result) => *result,
            None => return Err(GameError::InvalidMove),
        };

        match self.board.apply(chessmove) {
            Ok(_capture) => {
                self.save_move(chessmove);
                Ok(chessmove)
            }
            Err(error) => Err(GameError::BoardError { error }),
        }
    }

    pub fn score(&mut self, current_turn: Color) -> f32 {
        evaluate::score(&mut self.board, &mut self.targets, current_turn)
    }

    pub fn fullmove_clock(&self) -> u8 {
        self.board.fullmove_clock()
    }

    pub fn last_searched_position_count(&self) -> u32 {
        self.searcher.last_searched_position_count
    }

    pub fn last_cache_hit_count(&self) -> u32 {
        self.searcher.last_cache_hit_count
    }

    pub fn last_alpha_beta_termination_count(&self) -> u32 {
        self.searcher.last_alpha_beta_termination_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::piece::Piece;
    use crate::board::{square, ALL_CASTLE_RIGHTS};
    use crate::chess_move;

    #[test]
    fn test_score() {
        let mut game = Game::new(0);
        game.make_move(square::E2, square::E4).unwrap();
        game.board.next_turn();
        assert!(game.check_game_over_for_current_turn().is_none());
    }

    #[test]
    fn test_checkmate() {
        let mut game = Game::new(0);
        game.make_move(square::F2, square::F3).unwrap();
        game.board.next_turn();
        game.make_move(square::E7, square::E6).unwrap();
        game.board.next_turn();
        game.make_move(square::G2, square::G4).unwrap();
        game.board.next_turn();
        game.make_move(square::D8, square::H4).unwrap();
        game.board.next_turn();
        println!("Testing board:\n{}", game.board);
        matches!(
            game.check_game_over_for_current_turn(),
            Some(GameEnding::Checkmate)
        );
    }

    #[test]
    fn test_draw_from_repetition() {
        let mut board = Board::new();
        board.put(square::A1, Piece::Rook, Color::White).unwrap();
        board.put(square::A2, Piece::King, Color::White).unwrap();
        board.put(square::H7, Piece::Rook, Color::Black).unwrap();
        board.put(square::H8, Piece::King, Color::Black).unwrap();
        board.set_turn(Color::White);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        // make sure starting position has been counted
        board.update_position_hash();
        board.count_current_position();

        let mut game = Game::from_board(board, 0);
        println!("Testing board:\n{}", game.board);

        game.board
            .apply(chess_move!(square::A2, square::A3))
            .unwrap();
        game.board.next_turn();
        game.board
            .apply(chess_move!(square::H8, square::G8))
            .unwrap();
        game.board.next_turn();
        game.board
            .apply(chess_move!(square::A3, square::A2))
            .unwrap();
        game.board.next_turn();
        game.board
            .apply(chess_move!(square::G8, square::H8))
            .unwrap();
        game.board.next_turn();

        // back in starting position for second time
        matches!(
            game.check_game_over_for_current_turn(),
            Some(GameEnding::Draw)
        );

        game.board
            .apply(chess_move!(square::A2, square::A3))
            .unwrap();
        game.board.next_turn();
        game.board
            .apply(chess_move!(square::H8, square::G8))
            .unwrap();
        game.board.next_turn();
        game.board
            .apply(chess_move!(square::A3, square::A2))
            .unwrap();
        game.board.next_turn();
        game.board
            .apply(chess_move!(square::G8, square::H8))
            .unwrap();
        game.board.next_turn();

        // back in starting position for third time, should be draw
        matches!(
            game.check_game_over_for_current_turn(),
            Some(GameEnding::Draw)
        );
    }
}
