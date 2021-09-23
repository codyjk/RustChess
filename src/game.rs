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
        Self::from_board(Board::starting_position())
    }

    pub fn from_board(board: Board) -> Self {
        let mut ray_table = RayTable::new();
        ray_table.populate();

        Self {
            board: board,
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

    pub fn score(&self) -> f32 {
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
        let current_turn = self.board.turn();
        let candidates = moves::generate(&mut self.board, current_turn, &self.ray_table);
        if candidates.len() == 0 {
            return Err(GameError::NoAvailableMoves);
        }

        let mut best_move = None;
        let alpha = f32::NEG_INFINITY;
        let beta = f32::INFINITY;

        if current_turn.maximize_score() {
            let mut best_score = f32::NEG_INFINITY;
            for chessmove in candidates {
                self.board.apply(chessmove).unwrap();
                self.board.next_turn();
                let score =
                    minimax_alpha_beta(alpha, beta, depth, &mut self.board, &self.ray_table);
                self.board.undo(chessmove).unwrap();
                self.board.next_turn();

                if score >= best_score {
                    best_score = score;
                    best_move = Some(chessmove);
                }
            }
        } else {
            let mut best_score = f32::INFINITY;
            for chessmove in candidates {
                self.board.apply(chessmove).unwrap();
                self.board.next_turn();
                let score =
                    minimax_alpha_beta(alpha, beta, depth, &mut self.board, &self.ray_table);
                self.board.undo(chessmove).unwrap();
                self.board.next_turn();

                if score <= best_score {
                    best_score = score;
                    best_move = Some(chessmove);
                }
            }
        }

        match self.board.apply(best_move.unwrap()) {
            Ok(_capture) => Ok(best_move.unwrap()),
            Err(error) => Err(GameError::BoardError { error: error }),
        }
    }
}

fn minimax_alpha_beta(
    mut alpha: f32,
    mut beta: f32,
    depth: u8,
    board: &mut Board,
    ray_table: &RayTable,
) -> f32 {
    let current_turn = board.turn();
    let candidates = moves::generate(board, current_turn, ray_table);

    if depth == 0 || candidates.len() == 0 {
        return score(board, ray_table, current_turn);
    }

    if current_turn.maximize_score() {
        let mut best_score = f32::NEG_INFINITY;
        for chessmove in candidates {
            board.apply(chessmove).unwrap();
            board.next_turn();
            let score = minimax_alpha_beta(alpha, beta, depth - 1, board, ray_table);
            board.undo(chessmove).unwrap();
            board.next_turn();

            if score > best_score {
                best_score = score;
            }

            if score > alpha {
                alpha = score;
            }

            if score >= beta {
                break;
            }
        }
        best_score
    } else {
        let mut best_score = f32::INFINITY;
        for chessmove in candidates {
            board.apply(chessmove).unwrap();
            board.next_turn();
            let score = minimax_alpha_beta(alpha, beta, depth - 1, board, ray_table);
            board.undo(chessmove).unwrap();
            board.next_turn();

            if score < best_score {
                best_score = score;
            }

            if score < beta {
                beta = score;
            }

            if score <= alpha {
                break;
            }
        }
        best_score
    }
}

fn score(board: &mut Board, ray_table: &RayTable, current_turn: Color) -> f32 {
    match (game_ending(board, ray_table, current_turn), current_turn) {
        (Some(GameEnding::Checkmate), Color::White) => return f32::NEG_INFINITY,
        (Some(GameEnding::Checkmate), Color::Black) => return f32::INFINITY,
        (Some(GameEnding::Stalemate), Color::White) => return f32::NEG_INFINITY,
        (Some(GameEnding::Stalemate), Color::Black) => return f32::INFINITY,
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
    use crate::board::{square, ALL_CASTLE_RIGHTS};

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

    #[test]
    fn test_find_mate_in_1_white() {
        let mut board = Board::new();
        board.put(square::C2, Piece::King, Color::White).unwrap();
        board.put(square::A2, Piece::King, Color::Black).unwrap();
        board.put(square::B8, Piece::Queen, Color::White).unwrap();
        board.set_turn(Color::White);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        let mut game = Game::from_board(board);
        println!("Testing board:\n{}", game.render_board());

        let chessmove = game.make_alpha_beta_best_move(1).unwrap();
        let valid_checkmates = vec![
            ChessMove::new(square::B8, square::B2, None),
            ChessMove::new(square::B8, square::A8, None),
            ChessMove::new(square::B8, square::A7, None),
        ];
        assert!(valid_checkmates.contains(&chessmove));
    }

    #[test]
    fn test_find_mate_in_1_black() {
        let mut board = Board::new();
        board.put(square::C2, Piece::King, Color::Black).unwrap();
        board.put(square::A2, Piece::King, Color::White).unwrap();
        board.put(square::B8, Piece::Queen, Color::Black).unwrap();
        board.set_turn(Color::Black);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        let mut game = Game::from_board(board);
        println!("Testing board:\n{}", game.render_board());

        let chessmove = game.make_alpha_beta_best_move(1).unwrap();
        let valid_checkmates = vec![
            ChessMove::new(square::B8, square::B2, None),
            ChessMove::new(square::B8, square::A8, None),
            ChessMove::new(square::B8, square::A7, None),
        ];
        assert!(valid_checkmates.contains(&chessmove));
    }

    #[test]
    fn test_find_back_rank_mate_in_2_white() {
        let mut board = Board::new();
        board.put(square::A7, Piece::Pawn, Color::Black).unwrap();
        board.put(square::B7, Piece::Pawn, Color::Black).unwrap();
        board.put(square::C7, Piece::Pawn, Color::Black).unwrap();
        board.put(square::B8, Piece::King, Color::Black).unwrap();
        board.put(square::H8, Piece::Rook, Color::Black).unwrap();
        board.put(square::D1, Piece::Rook, Color::White).unwrap();
        board.put(square::D2, Piece::Queen, Color::White).unwrap();
        board.put(square::A1, Piece::King, Color::White).unwrap();
        board.set_turn(Color::White);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        let mut game = Game::from_board(board);
        println!("Testing board:\n{}", game.render_board());

        let expected_moves = [
            ChessMove::new(square::D2, square::D8, None),
            ChessMove::new(square::H8, square::D8, Some((Piece::Queen, Color::White))),
            ChessMove::new(square::D1, square::D8, Some((Piece::Rook, Color::Black))),
        ];

        let move1 = game.make_alpha_beta_best_move(2).unwrap();
        game.next_turn();
        assert_eq!(expected_moves[0], move1);
        println!("Testing board:\n{}", game.render_board());

        let move2 = game.make_alpha_beta_best_move(1).unwrap();
        game.next_turn();
        assert_eq!(expected_moves[1], move2);
        println!("Testing board:\n{}", game.render_board());

        let move3 = game.make_alpha_beta_best_move(0).unwrap();
        game.next_turn();
        assert_eq!(expected_moves[2], move3);
        println!("Testing board:\n{}", game.render_board());
    }

    #[test]
    fn test_find_back_rank_mate_in_2_black() {
        let mut board = Board::new();
        board.put(square::F2, Piece::Pawn, Color::White).unwrap();
        board.put(square::G2, Piece::Pawn, Color::White).unwrap();
        board.put(square::H2, Piece::Pawn, Color::White).unwrap();
        board.put(square::G1, Piece::King, Color::White).unwrap();
        board.put(square::A1, Piece::Rook, Color::White).unwrap();
        board.put(square::E8, Piece::Rook, Color::Black).unwrap();
        board.put(square::E7, Piece::Queen, Color::Black).unwrap();
        board.put(square::H8, Piece::King, Color::Black).unwrap();
        board.set_turn(Color::Black);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        let mut game = Game::from_board(board);
        println!("Testing board:\n{}", game.render_board());

        let expected_moves = [
            ChessMove::new(square::E7, square::E1, None),
            ChessMove::new(square::A1, square::E1, Some((Piece::Queen, Color::Black))),
            ChessMove::new(square::E8, square::E1, Some((Piece::Rook, Color::White))),
        ];

        let move1 = game.make_alpha_beta_best_move(2).unwrap();
        game.next_turn();
        assert_eq!(expected_moves[0], move1);
        println!("Testing board:\n{}", game.render_board());

        let move2 = game.make_alpha_beta_best_move(1).unwrap();
        game.next_turn();
        assert_eq!(expected_moves[1], move2);
        println!("Testing board:\n{}", game.render_board());

        let move3 = game.make_alpha_beta_best_move(0).unwrap();
        game.next_turn();
        assert_eq!(expected_moves[2], move3);
        println!("Testing board:\n{}", game.render_board());
    }
}
