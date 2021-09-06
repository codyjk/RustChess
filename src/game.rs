pub mod command;
pub mod modes;

use crate::board::color::Color;
use crate::board::error::BoardError;
use crate::board::Board;
use crate::moves;
use crate::moves::chess_move::ChessMove;
use crate::moves::ray_table::RayTable;
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

    pub fn render_board(&self) -> String {
        self.board.to_ascii()
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

    pub fn make_shallow_material_optimal_move(&mut self) -> Result<ChessMove, GameError> {
        let turn = self.turn();
        let candidates = moves::generate(&mut self.board, turn, &self.ray_table);
        if candidates.len() == 0 {
            return Err(GameError::NoAvailableMoves);
        }

        let material_values: Vec<i32> = candidates
            .iter()
            .map(|&chessmove| {
                self.board.apply(chessmove).unwrap();
                let material = self.board.material_value();
                self.board.undo(chessmove).unwrap();
                material
            })
            .collect();

        let material_target = match turn {
            Color::White => material_values.iter().max().unwrap(),
            Color::Black => material_values.iter().min().unwrap(),
        };

        let move_material = candidates.iter().zip(material_values.iter());
        let best_moves: Vec<&ChessMove> = move_material
            .filter(|&(_chessmove, material)| material == material_target)
            .map(|(chessmove, _material)| chessmove)
            .collect();
        let rng = rand::thread_rng().gen_range(0..best_moves.len());
        let best_move = best_moves[rng];

        match self.board.apply(*best_move) {
            Ok(_capture) => Ok(*best_move),
            Err(error) => Err(GameError::BoardError { error: error }),
        }
    }

    pub fn make_minimax_best_move(&mut self) -> Result<ChessMove, GameError> {
        let turn = self.turn();
        let (maybe_chessmove, _score) =
            minimax_select_move(3, &mut self.board, &self.ray_table, turn);

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

fn minimax_select_move(
    depth: u8,
    board: &mut Board,
    ray_table: &RayTable,
    current_turn: Color,
) -> (Option<ChessMove>, i32) {
    if depth == 0 {
        let score = board.material_value();
        return (None, score);
    }

    let mut best_score = match current_turn {
        Color::White => i32::MIN,
        Color::Black => i32::MAX,
    };
    let mut best_move = None;

    let candidates = moves::generate(board, current_turn, ray_table);
    for chessmove in candidates {
        board.apply(chessmove).unwrap();
        let (_deep_best_move, score) =
            minimax_select_move(depth - 1, board, ray_table, current_turn.opposite());
        board.undo(chessmove).unwrap();

        let is_best_move = match current_turn {
            Color::White => score > best_score,
            Color::Black => score < best_score,
        };

        if is_best_move {
            best_score = score;
            best_move = Some(chessmove);
        }
    }

    (best_move, best_score)
}
