pub mod command;
pub mod modes;

use crate::board::color::Color;
use crate::board::error::BoardError;
use crate::board::piece::Piece;
use crate::board::Board;
use crate::moves::chess_move::ChessMove;
use crate::moves::ray_table::RayTable;
use crate::moves::{self, targets};
use rand::{self, Rng};
use thiserror::Error;

pub struct Game {
    board: Board,
    ray_table: RayTable,
}

#[derive(Error, Debug)]
pub enum GameError {
    #[error("that is not a valid move")]
    InvalidMove,
    #[error("no available moves")]
    NoAvailableMoves,
    #[error("board error: {error:?}")]
    BoardError { error: BoardError },
}

#[derive(Debug)]
pub enum GameEnding {
    Checkmate,
    Stalemate,
}

impl Game {
    pub fn new() -> Self {
        let mut ray_table = RayTable::new();
        ray_table.populate();

        Self {
            board: Board::starting_position(),
            ray_table: ray_table,
        }
    }

    pub fn turn(&self) -> Color {
        self.board.turn()
    }

    pub fn next_turn(&mut self) -> Color {
        self.board.next_turn()
    }

    pub fn halfmove_clock(&self) -> u8 {
        self.board.peek_halfmove_clock()
    }

    pub fn fullmove_clock(&self) -> u8 {
        self.board.fullmove_clock()
    }

    pub fn score(&self) -> i32 {
        self.board.score()
    }

    pub fn check_game_over_for_current_turn(&mut self) -> Option<GameEnding> {
        let turn = self.board.turn();
        game_ending(&mut self.board, &self.ray_table, turn)
    }

    pub fn render_board(&self) -> String {
        self.board.to_ascii()
    }

    pub fn fen(&self) -> String {
        self.board.to_fen()
    }

    pub fn make_move(&mut self, from_square: u64, to_square: u64) -> Result<ChessMove, GameError> {
        let turn = self.turn();
        let candidates = moves::generate(&mut self.board, turn, &self.ray_table);
        let maybe_chessmove = candidates
            .iter()
            .find(|&m| m.from_square() == from_square && m.to_square() == to_square);
        let chessmove = match maybe_chessmove {
            Some(result) => *result,
            None => return Err(GameError::InvalidMove),
        };
        match self.board.apply(chessmove) {
            Ok(_capture) => Ok(chessmove),
            Err(error) => Err(GameError::BoardError { error: error }),
        }
    }

    pub fn make_random_move(&mut self) -> Result<ChessMove, GameError> {
        let turn = self.turn();
        let candidates = moves::generate(&mut self.board, turn, &self.ray_table);
        let chessmove = match candidates.len() {
            0 => return Err(GameError::NoAvailableMoves),
            _ => {
                let rng = rand::thread_rng().gen_range(0..candidates.len());
                candidates[rng]
            }
        };
        match self.board.apply(chessmove) {
            Ok(_capture) => Ok(chessmove),
            Err(error) => Err(GameError::BoardError { error: error }),
        }
    }

    pub fn make_alpha_beta_best_move(&mut self, depth: u8) -> Result<ChessMove, GameError> {
        let turn = self.turn();
        let alpha_beta_selection = match turn {
            Color::White => alpha_beta_max,
            Color::Black => alpha_beta_min,
        };
        let (maybe_chessmove, _score) = alpha_beta_selection(
            i32::MIN,
            i32::MAX,
            depth,
            &mut self.board,
            &self.ray_table,
            turn,
        );

        let best_move = match maybe_chessmove {
            Some(chessmove) => chessmove,
            None => return Err(GameError::NoAvailableMoves),
        };

        match self.board.apply(best_move) {
            Ok(_capture) => Ok(best_move),
            Err(error) => Err(GameError::BoardError { error: error }),
        }
    }
}

fn alpha_beta_max(
    alpha: i32,
    beta: i32,
    depth: u8,
    board: &mut Board,
    ray_table: &RayTable,
    current_turn: Color,
) -> (Option<ChessMove>, i32) {
    if depth == 0 {
        return (None, score(board, ray_table, current_turn));
    }

    let mut new_alpha = alpha;
    let mut best_move = None;

    let candidates = moves::generate(board, current_turn, ray_table);

    for chessmove in candidates {
        board.apply(chessmove).unwrap();
        let (_deep_best_move, score) = alpha_beta_min(
            new_alpha,
            beta,
            depth - 1,
            board,
            ray_table,
            current_turn.opposite(),
        );
        board.undo(chessmove).unwrap();

        if score >= beta {
            return (None, beta);
        }

        if score > new_alpha {
            new_alpha = score;
            best_move = Some(chessmove);
        }
    }

    (best_move, new_alpha)
}

fn alpha_beta_min(
    alpha: i32,
    beta: i32,
    depth: u8,
    board: &mut Board,
    ray_table: &RayTable,
    current_turn: Color,
) -> (Option<ChessMove>, i32) {
    if depth == 0 {
        return (None, -1 * score(board, ray_table, current_turn));
    }

    let mut new_beta = beta;
    let mut best_move = None;

    let candidates = moves::generate(board, current_turn, ray_table);

    for chessmove in candidates {
        board.apply(chessmove).unwrap();
        let (_deep_best_move, score) = alpha_beta_max(
            alpha,
            new_beta,
            depth - 1,
            board,
            ray_table,
            current_turn.opposite(),
        );
        board.undo(chessmove).unwrap();

        if score <= alpha {
            return (None, alpha);
        }

        if score < new_beta {
            new_beta = score;
            best_move = Some(chessmove);
        }
    }

    (best_move, new_beta)
}

fn score(board: &mut Board, ray_table: &RayTable, current_turn: Color) -> i32 {
    match (game_ending(board, ray_table, current_turn), current_turn) {
        (Some(GameEnding::Checkmate), Color::White) => return i32::MIN,
        (Some(GameEnding::Checkmate), Color::Black) => return i32::MAX,
        (Some(GameEnding::Stalemate), Color::White) => return i32::MAX,
        (Some(GameEnding::Stalemate), Color::Black) => return i32::MIN,
        _ => (),
    };

    board.score()
}

fn current_player_is_in_check(board: &Board, ray_table: &RayTable) -> bool {
    let current_player = board.turn();
    let king = board.pieces(current_player).locate(Piece::King);
    let attacked_squares =
        targets::generate_attack_targets(board, current_player.opposite(), ray_table);

    king & attacked_squares > 0
}

pub fn game_ending(
    board: &mut Board,
    ray_table: &RayTable,
    current_turn: Color,
) -> Option<GameEnding> {
    let candidates = moves::generate(board, current_turn, ray_table);
    let check = current_player_is_in_check(board, ray_table);

    if candidates.len() == 0 {
        if check {
            return Some(GameEnding::Checkmate);
        } else {
            return Some(GameEnding::Stalemate);
        }
    }

    return None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::square;

    #[test]
    fn test_score() {
        let mut game = Game::new();
        game.make_move(square::E2, square::E4).unwrap();
        game.next_turn();
        assert!(game.check_game_over_for_current_turn().is_none());
    }

    #[test]
    fn test_checkmate() {
        let mut game = Game::new();
        game.make_move(square::F2, square::F3).unwrap();
        game.next_turn();
        game.make_move(square::E7, square::E6).unwrap();
        game.next_turn();
        game.make_move(square::G2, square::G4).unwrap();
        game.next_turn();
        game.make_move(square::D8, square::H4).unwrap();
        game.next_turn();
        println!("Testing board:\n{}", game.render_board());
        let checkmate = match game.check_game_over_for_current_turn() {
            Some(GameEnding::Checkmate) => true,
            _ => false,
        };
        assert!(checkmate);
    }
}
