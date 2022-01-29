use crate::board::Board;
use crate::moves::chess_move::ChessMove;
use crate::moves::targets::Targets;
use crate::{evaluate, moves};
use ahash::AHashMap;
use thiserror::Error;

type SearchNode = (u64, u8); // (boardstate_hash, depth)
type SearchResult = f32; // best_score

pub struct Searcher {
    search_depth: u8,
    search_result_cache: AHashMap<SearchNode, SearchResult>,
    pub last_searched_position_count: u32,
    pub last_cache_hit_count: u32,
    pub last_alpha_beta_termination_count: u32,
}

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("no available moves")]
    NoAvailableMoves,
}

impl Searcher {
    pub fn new(depth: u8) -> Self {
        Self {
            search_depth: depth,
            search_result_cache: AHashMap::new(),
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

        let current_turn = board.turn();
        let candidates = moves::generate(board, current_turn, targets);
        if candidates.len() == 0 {
            return Err(SearchError::NoAvailableMoves);
        }

        let mut best_move = None;
        let alpha = f32::NEG_INFINITY;
        let beta = f32::INFINITY;

        if current_turn.maximize_score() {
            let mut best_score = f32::NEG_INFINITY;
            for chessmove in candidates {
                board.apply(chessmove).unwrap();
                board.next_turn();
                let score = self.minimax_alpha_beta(alpha, beta, self.search_depth, board, targets);
                board.undo(chessmove).unwrap();
                board.next_turn();

                if score >= best_score {
                    best_score = score;
                    best_move = Some(chessmove);
                }
            }
        } else {
            let mut best_score = f32::INFINITY;
            for chessmove in candidates {
                board.apply(chessmove).unwrap();
                board.next_turn();
                let score = self.minimax_alpha_beta(alpha, beta, self.search_depth, board, targets);
                board.undo(chessmove).unwrap();
                board.next_turn();

                if score <= best_score {
                    best_score = score;
                    best_move = Some(chessmove);
                }
            }
        }

        Ok(best_move.unwrap())
    }

    fn minimax_alpha_beta(
        &mut self,
        mut alpha: f32,
        mut beta: f32,
        depth: u8,
        board: &mut Board,
        targets: &mut Targets,
    ) -> f32 {
        let current_turn = board.turn();
        let candidates = moves::generate(board, current_turn, targets);
        let search_node: SearchNode = (board.current_boardstate_hash(), depth);
        let mut best_score;
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

        if current_turn.maximize_score() {
            best_score = f32::NEG_INFINITY;
            for chessmove in candidates {
                board.apply(chessmove).unwrap();
                board.next_turn();
                let score = self.minimax_alpha_beta(alpha, beta, depth - 1, board, targets);
                board.undo(chessmove).unwrap();
                board.next_turn();

                if score > best_score {
                    best_score = score;
                }

                if score > alpha {
                    alpha = score;
                }

                if score >= beta {
                    self.last_alpha_beta_termination_count += 1;
                    break;
                }
            }
        } else {
            best_score = f32::INFINITY;
            for chessmove in candidates {
                board.apply(chessmove).unwrap();
                board.next_turn();
                let score = self.minimax_alpha_beta(alpha, beta, depth - 1, board, targets);
                board.undo(chessmove).unwrap();
                board.next_turn();

                if score < best_score {
                    best_score = score;
                }

                if score < beta {
                    beta = score;
                }

                if score <= alpha {
                    self.last_alpha_beta_termination_count += 1;
                    break;
                }
            }
        }

        self.search_result_cache.insert(search_node, best_score);

        best_score
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
