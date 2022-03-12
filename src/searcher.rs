use crate::board::Board;
use crate::moves::chess_move::ChessMove;
use crate::moves::targets::Targets;
use crate::{evaluate, moves};
use rustc_hash::FxHashMap;
use thiserror::Error;

type SearchNode = (u64, u8, u8); // (board_hash, depth, current_turn)
type SearchResult = f32; // best_score

pub struct Searcher {
    search_depth: u8,
    search_result_cache: FxHashMap<SearchNode, SearchResult>,
    alpha: f32,
    beta: f32,
    pub last_searched_position_count: u32,
    pub last_cache_hit_count: u32,
    pub last_alpha_beta_termination_count: u32,
}

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("no available moves")]
    NoAvailableMoves,
}

type ScoreComparator = fn(f32, f32) -> bool;
fn gte(a: f32, b: f32) -> bool {
    a >= b
}
fn lte(a: f32, b: f32) -> bool {
    a <= b
}
fn gt(a: f32, b: f32) -> bool {
    a > b
}
fn lt(a: f32, b: f32) -> bool {
    a < b
}

impl Searcher {
    pub fn new(depth: u8) -> Self {
        Self {
            search_depth: depth,
            search_result_cache: FxHashMap::default(),
            alpha: f32::NEG_INFINITY,
            beta: f32::INFINITY,
            last_searched_position_count: 0,
            last_cache_hit_count: 0,
            last_alpha_beta_termination_count: 0,
        }
    }

    pub fn search(
        &mut self,
        board: &mut Board,
        targets: &mut Targets,
    ) -> Result<ChessMove, SearchError> {
        self.last_searched_position_count = 0;
        self.last_cache_hit_count = 0;
        self.last_alpha_beta_termination_count = 0;
        self.alpha = f32::NEG_INFINITY;
        self.beta = f32::INFINITY;

        let current_turn = board.turn();
        let candidates = moves::generate(board, current_turn, targets);
        if candidates.len() == 0 {
            return Err(SearchError::NoAvailableMoves);
        }

        let mut best_move = None;

        // set search context relative to the player who we are maximizing for
        let (mut best_score, cmp): (f32, ScoreComparator) = match current_turn.maximize_score() {
            true => (f32::NEG_INFINITY, gte),
            false => (f32::INFINITY, lte),
        };

        for chessmove in candidates {
            board.apply(chessmove).unwrap();
            board.next_turn();
            let score = self.minimax_alpha_beta(self.search_depth, board, targets);
            board.undo(chessmove).unwrap();
            board.next_turn();

            if cmp(score, best_score) {
                best_score = score;
                best_move = Some(chessmove);
            }
        }

        Ok(best_move.unwrap())
    }

    fn minimax_alpha_beta(&mut self, depth: u8, board: &mut Board, targets: &mut Targets) -> f32 {
        let current_turn = board.turn();
        let candidates = moves::generate(board, current_turn, targets);
        let search_node = (board.current_position_hash(), depth, current_turn as u8);
        self.last_searched_position_count += 1;

        match self.search_result_cache.get(&search_node) {
            Some(&prev_best_score) => {
                self.last_cache_hit_count += 1;
                return prev_best_score;
            }
            None => (),
        };

        if depth == 0 || candidates.len() == 0 {
            return evaluate::score(board, targets, current_turn);
        }

        // set search context relative to the player who we are maximizing for
        let (mut best_score, rel_alpha, rel_beta, cmp, cmpe): (
            f32,
            f32,
            f32,
            ScoreComparator,
            ScoreComparator,
        ) = match current_turn.maximize_score() {
            true => (f32::NEG_INFINITY, self.alpha, self.beta, gt, gte),
            false => (f32::INFINITY, self.beta, self.alpha, lt, lte),
        };

        for chessmove in candidates {
            board.apply(chessmove).unwrap();
            board.next_turn();
            let score = self.minimax_alpha_beta(depth - 1, board, targets);
            board.undo(chessmove).unwrap();
            board.next_turn();

            if cmp(score, best_score) {
                best_score = score;
            }

            if cmp(score, rel_alpha) {
                if current_turn.maximize_score() {
                    self.set_alpha(score);
                } else {
                    self.set_beta(score);
                }
            }

            if cmpe(score, rel_beta) {
                self.last_alpha_beta_termination_count += 1;
                break;
            }
        }

        self.search_result_cache.insert(search_node, best_score);

        best_score
    }

    fn set_alpha(&mut self, to: f32) {
        self.alpha = to
    }

    fn set_beta(&mut self, to: f32) {
        self.beta = to
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::color::Color;
    use crate::board::piece::Piece;
    use crate::board::{square, ALL_CASTLE_RIGHTS};

    #[test]
    fn test_find_mate_in_1_white() {
        let mut board = Board::new();
        let mut targets = Targets::new();
        let mut searcher = Searcher::new(1);

        board.put(square::C2, Piece::King, Color::White).unwrap();
        board.put(square::A2, Piece::King, Color::Black).unwrap();
        board.put(square::B8, Piece::Queen, Color::White).unwrap();
        board.set_turn(Color::White);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);
        println!("Testing board:\n{}", board);

        let chessmove = searcher.search(&mut board, &mut targets).unwrap();
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
        let mut targets = Targets::new();
        let mut searcher = Searcher::new(1);

        board.put(square::C2, Piece::King, Color::Black).unwrap();
        board.put(square::A2, Piece::King, Color::White).unwrap();
        board.put(square::B8, Piece::Queen, Color::Black).unwrap();
        board.set_turn(Color::Black);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        println!("Testing board:\n{}", board);

        let chessmove = searcher.search(&mut board, &mut targets).unwrap();

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
        let mut targets = Targets::new();
        let mut searcher = Searcher::new(2);

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

        println!("Testing board:\n{}", board);

        let expected_moves = [
            ChessMove::new(square::D2, square::D8, None),
            ChessMove::new(square::H8, square::D8, Some((Piece::Queen, Color::White))),
            ChessMove::new(square::D1, square::D8, Some((Piece::Rook, Color::Black))),
        ];

        let move1 = searcher.search(&mut board, &mut targets).unwrap();
        board.apply(move1).unwrap();
        board.next_turn();
        assert_eq!(expected_moves[0], move1);
        println!("Testing board:\n{}", board);

        let move2 = searcher.search(&mut board, &mut targets).unwrap();
        board.apply(move2).unwrap();
        board.next_turn();
        assert_eq!(expected_moves[1], move2);
        println!("Testing board:\n{}", board);

        let move3 = searcher.search(&mut board, &mut targets).unwrap();
        board.apply(move3).unwrap();
        board.next_turn();
        assert_eq!(expected_moves[2], move3);
        println!("Testing board:\n{}", board);
    }

    #[test]
    fn test_find_back_rank_mate_in_2_black() {
        let mut board = Board::new();
        let mut targets = Targets::new();
        let mut searcher = Searcher::new(2);

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

        println!("Testing board:\n{}", board);

        let expected_moves = [
            ChessMove::new(square::E7, square::E1, None),
            ChessMove::new(square::A1, square::E1, Some((Piece::Queen, Color::Black))),
            ChessMove::new(square::E8, square::E1, Some((Piece::Rook, Color::White))),
        ];

        let move1 = searcher.search(&mut board, &mut targets).unwrap();
        board.apply(move1).unwrap();
        board.next_turn();
        assert_eq!(expected_moves[0], move1);
        println!("Testing board:\n{}", board);

        let move2 = searcher.search(&mut board, &mut targets).unwrap();
        board.apply(move2).unwrap();
        board.next_turn();
        assert_eq!(expected_moves[1], move2);
        println!("Testing board:\n{}", board);

        let move3 = searcher.search(&mut board, &mut targets).unwrap();
        board.apply(move3).unwrap();
        board.next_turn();
        assert_eq!(expected_moves[2], move3);
        println!("Testing board:\n{}", board);
    }
}
