pub mod command;
pub mod modes;

use crate::board::color::Color;
use crate::board::error::BoardError;
use crate::board::Board;
use crate::book::{generate_opening_book, Book};
use crate::evaluate::{self, GameEnding};
use crate::move_generator::generate_valid_moves;
use crate::move_generator::targets::Targets;
use crate::searcher::{SearchError, Searcher};
use rand::{self, Rng};
use thiserror::Error;

pub type BoardMove = (u64, u64);

pub struct Game {
    board: Board,
    move_history: Vec<BoardMove>,
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
            move_history: Vec::new(),
            book: generate_opening_book(),
            targets: Targets::new(),
            searcher: Searcher::new(search_depth),
        }
    }

    pub fn check_game_over_for_current_turn(&mut self) -> Option<GameEnding> {
        let turn = self.board.turn();
        evaluate::game_ending(&mut self.board, &mut self.targets, turn)
    }

    pub fn save_move(&mut self, board_move: BoardMove) {
        self.move_history.push(board_move);
    }

    pub fn most_recent_move(&self) -> Option<BoardMove> {
        self.move_history.iter().last().copied()
    }

    pub fn make_move(&mut self, from_square: u64, to_square: u64) -> Result<BoardMove, GameError> {
        let turn = self.board.turn();
        let candidates = generate_valid_moves(&mut self.board, turn, &mut self.targets);
        let chess_move = candidates
            .iter()
            .find(|m| m.from_square() == from_square && m.to_square() == to_square)
            .ok_or(GameError::InvalidMove)?;
        match chess_move.apply(&mut self.board) {
            Ok(_capture) => {
                self.save_move((from_square, to_square));
                Ok((from_square, to_square))
            }
            Err(error) => Err(GameError::BoardError { error }),
        }
    }

    pub fn make_alpha_beta_best_move(&mut self) -> Result<BoardMove, GameError> {
        let best_move = match self.searcher.search(&mut self.board, &mut self.targets) {
            Ok(mv) => mv,
            Err(err) => return Err(GameError::SearchError { error: err }),
        };

        match best_move.apply(&mut self.board) {
            Ok(_capture) => {
                self.save_move((best_move.from_square(), best_move.to_square()));
                Ok((best_move.from_square(), best_move.to_square()))
            }
            Err(error) => Err(GameError::BoardError { error }),
        }
    }

    pub fn make_waterfall_book_then_alpha_beta_move(&mut self) -> Result<BoardMove, GameError> {
        let current_turn = self.board.turn();
        let line = self.move_history.iter().map(|bm| *bm).collect();
        let book_moves = self.book.get_next_moves(line);

        if book_moves.is_empty() {
            return self.make_alpha_beta_best_move();
        }

        let rng = rand::thread_rng().gen_range(0..book_moves.len());
        let (from_square, to_square) = book_moves[rng];
        let candidates = generate_valid_moves(&mut self.board, current_turn, &mut self.targets);

        let maybe_chess_move = candidates
            .iter()
            .find(|m| m.from_square() == from_square && m.to_square() == to_square);

        let chess_move = match maybe_chess_move {
            Some(result) => result,
            None => return Err(GameError::InvalidMove),
        };

        match chess_move.apply(&mut self.board) {
            Ok(_capture) => {
                self.save_move((from_square, to_square));
                Ok((from_square, to_square))
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
    use crate::board::castle_rights::ALL_CASTLE_RIGHTS;
    use crate::board::piece::Piece;
    use crate::board::square;
    use crate::chess_move::standard::StandardChessMove;
    use crate::chess_move::ChessMove;
    use crate::std_move;

    #[test]
    fn test_score() {
        let mut game = Game::new(0);
        game.make_move(square::E2, square::E4).unwrap();
        game.board.toggle_turn();
        assert!(game.check_game_over_for_current_turn().is_none());
    }

    #[test]
    fn test_checkmate() {
        let mut game = Game::new(0);
        game.make_move(square::F2, square::F3).unwrap();
        game.board.toggle_turn();
        game.make_move(square::E7, square::E6).unwrap();
        game.board.toggle_turn();
        game.make_move(square::G2, square::G4).unwrap();
        game.board.toggle_turn();
        game.make_move(square::D8, square::H4).unwrap();
        game.board.toggle_turn();
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
        board.count_current_position();

        let mut game = Game::from_board(board, 0);
        println!("Testing board:\n{}", game.board);

        let first_moves = vec![
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

        let second_moves = vec![
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
