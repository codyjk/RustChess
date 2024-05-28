use crate::board::Board;
use crate::chess_move::ChessMove;
use crate::move_generator::MoveGenerator;
use crate::{board::color::Color, evaluate};
use rustc_hash::FxHashMap;
use thiserror::Error;

type SearchNode = (u64, u8, u8); // (board_hash, depth, current_turn)
type SearchResult = i16; // best_score

pub struct Searcher {
    search_depth: u8,
    search_result_cache: FxHashMap<SearchNode, SearchResult>,
    searched_position_count: usize,
    cache_hit_count: usize,
    termination_count: usize,
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
            search_result_cache: FxHashMap::default(),
            searched_position_count: 0,
            cache_hit_count: 0,
            termination_count: 0,
        }
    }

    pub fn searched_position_count(&self) -> usize {
        self.searched_position_count
    }

    pub fn cache_hit_count(&self) -> usize {
        self.cache_hit_count
    }

    pub fn termination_count(&self) -> usize {
        self.termination_count
    }

    pub fn search(
        &mut self,
        board: &mut Board,
        move_generator: &mut MoveGenerator,
    ) -> Result<ChessMove, SearchError> {
        self.searched_position_count = 0;
        self.cache_hit_count = 0;
        self.termination_count = 0;

        let current_turn = board.turn();
        let candidates = move_generator.generate_moves(board, current_turn);

        if candidates.is_empty() {
            return Err(SearchError::NoAvailableMoves);
        }

        let scores: Vec<i16> = candidates
            .iter()
            .map(|chess_move| {
                chess_move.apply(board).unwrap();
                board.toggle_turn();
                let score = self.alpha_beta_max(
                    self.search_depth,
                    board,
                    move_generator,
                    i16::MIN,
                    i16::MAX,
                );
                chess_move.undo(board).unwrap();
                board.toggle_turn();
                score
            })
            .collect();

        let mut results = candidates
            .into_iter()
            .zip(scores.iter())
            .collect::<Vec<_>>();
        results.sort_by(|(_, score_a), (_, score_b)| score_a.partial_cmp(score_b).unwrap());
        results.reverse();
        let (best_move, _) = results.pop().unwrap();
        Ok(best_move)
    }

    fn alpha_beta_max(
        &mut self,
        depth: u8,
        board: &mut Board,
        move_generator: &mut MoveGenerator,
        mut alpha: i16,
        beta: i16,
    ) -> i16 {
        self.searched_position_count += 1;

        if depth == 0 {
            let score = evaluate::score(board, move_generator, board.turn());
            return score;
        }

        self.check_cache(board.current_position_hash(), depth, board.turn());
        let candidates = move_generator.generate_moves(board, board.turn());

        for chess_move in candidates.iter() {
            chess_move.apply(board).unwrap();
            board.toggle_turn();
            let score = self.alpha_beta_min(depth - 1, board, move_generator, alpha, beta);
            chess_move.undo(board).unwrap();
            board.toggle_turn();

            if score >= beta {
                self.termination_count += 1;
                self.set_cache(board.current_position_hash(), depth, board.turn(), beta);
                return beta;
            }

            if score > alpha {
                alpha = score;
            }
        }

        self.set_cache(board.current_position_hash(), depth, board.turn(), alpha);

        alpha
    }

    fn alpha_beta_min(
        &mut self,
        depth: u8,
        board: &mut Board,
        move_generator: &mut MoveGenerator,
        alpha: i16,
        mut beta: i16,
    ) -> i16 {
        self.searched_position_count += 1;

        if depth == 0 {
            let score = -evaluate::score(board, move_generator, board.turn());
            return score;
        }

        self.check_cache(board.current_position_hash(), depth, board.turn());
        let candidates = move_generator.generate_moves(board, board.turn());

        for chess_move in candidates.iter() {
            chess_move.apply(board).unwrap();
            board.toggle_turn();
            let score = self.alpha_beta_max(depth - 1, board, move_generator, alpha, beta);
            chess_move.undo(board).unwrap();
            board.toggle_turn();

            if score <= alpha {
                self.termination_count += 1;
                self.set_cache(board.current_position_hash(), depth, board.turn(), alpha);

                return alpha;
            }

            if score < beta {
                beta = score;
            }
        }

        self.set_cache(board.current_position_hash(), depth, board.turn(), beta);

        beta
    }

    fn set_cache(&mut self, position_hash: u64, depth: u8, current_turn: Color, score: i16) {
        let search_node = (position_hash, depth, current_turn as u8);
        self.search_result_cache.insert(search_node, score);
    }

    fn check_cache(&mut self, position_hash: u64, depth: u8, current_turn: Color) -> Option<i16> {
        let search_node = (position_hash, depth, current_turn as u8);
        match self.search_result_cache.get(&search_node) {
            Some(&prev_best_score) => {
                self.cache_hit_count += 1;
                Some(prev_best_score)
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::castle_rights::ALL_CASTLE_RIGHTS;
    use crate::board::color::Color;
    use crate::board::piece::Piece;
    use crate::board::square::*;
    use crate::chess_move::standard::StandardChessMove;
    use crate::std_move;

    #[test]
    fn test_find_mate_in_1_white() {
        let mut board = Board::new();
        let mut searcher = Searcher::new(1);
        let mut move_generator = MoveGenerator::new();

        board.put(C2, Piece::King, Color::White).unwrap();
        board.put(A2, Piece::King, Color::Black).unwrap();
        board.put(B8, Piece::Queen, Color::White).unwrap();
        board.set_turn(Color::White);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);
        println!("Testing board:\n{}", board);

        let chess_move = searcher.search(&mut board, &mut move_generator).unwrap();
        let valid_checkmates = vec![std_move!(B8, B2), std_move!(B8, A8), std_move!(B8, A7)];
        assert!(
            valid_checkmates.contains(&chess_move),
            "{} does not leed to checkmate",
            chess_move
        );
    }

    #[test]
    fn test_find_mate_in_1_black() {
        let mut board = Board::new();
        let mut searcher = Searcher::new(1);
        let mut move_generator = MoveGenerator::new();

        board.put(C2, Piece::King, Color::Black).unwrap();
        board.put(A2, Piece::King, Color::White).unwrap();
        board.put(B8, Piece::Queen, Color::Black).unwrap();
        board.set_turn(Color::Black);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        println!("Testing board:\n{}", board);

        let chess_move = searcher.search(&mut board, &mut move_generator).unwrap();

        let valid_checkmates = vec![std_move!(B8, B2), std_move!(B8, A8), std_move!(B8, A7)];
        assert!(valid_checkmates.contains(&chess_move));
    }

    #[test]
    fn test_find_back_rank_mate_in_2_white() {
        let mut board = Board::new();
        let mut searcher = Searcher::new(2);
        let mut move_generator = MoveGenerator::new();

        board.put(A7, Piece::Pawn, Color::Black).unwrap();
        board.put(B7, Piece::Pawn, Color::Black).unwrap();
        board.put(C7, Piece::Pawn, Color::Black).unwrap();
        board.put(B8, Piece::King, Color::Black).unwrap();
        board.put(H8, Piece::Rook, Color::Black).unwrap();
        board.put(D1, Piece::Rook, Color::White).unwrap();
        board.put(D2, Piece::Queen, Color::White).unwrap();
        board.put(A1, Piece::King, Color::White).unwrap();
        board.set_turn(Color::White);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        println!("Testing board:\n{}", board);

        let expected_moves = vec![
            std_move!(D2, D8),
            std_move!(H8, D8, (Piece::Queen, Color::White)),
            std_move!(D1, D8, (Piece::Rook, Color::Black)),
        ];
        let mut expected_move_iter = expected_moves.iter();

        let move1 = searcher.search(&mut board, &mut move_generator).unwrap();
        move1.apply(&mut board).unwrap();
        board.toggle_turn();
        assert_eq!(expected_move_iter.next().unwrap(), &move1);
        println!("Testing board:\n{}", board);

        let move2 = searcher.search(&mut board, &mut move_generator).unwrap();
        move2.apply(&mut board).unwrap();
        board.toggle_turn();
        assert_eq!(expected_move_iter.next().unwrap(), &move2);
        println!("Testing board:\n{}", board);

        let move3 = searcher.search(&mut board, &mut move_generator).unwrap();
        move3.apply(&mut board).unwrap();
        board.toggle_turn();
        assert_eq!(expected_move_iter.next().unwrap(), &move3);
        println!("Testing board:\n{}", board);
    }

    #[test]
    fn test_find_back_rank_mate_in_2_black() {
        let mut board = Board::new();
        let mut searcher = Searcher::new(3);
        let mut move_generator = MoveGenerator::new();

        board.put(F2, Piece::Pawn, Color::White).unwrap();
        board.put(G2, Piece::Pawn, Color::White).unwrap();
        board.put(H2, Piece::Pawn, Color::White).unwrap();
        board.put(G1, Piece::King, Color::White).unwrap();
        board.put(A1, Piece::Rook, Color::White).unwrap();
        board.put(E8, Piece::Rook, Color::Black).unwrap();
        board.put(E7, Piece::Queen, Color::Black).unwrap();
        board.put(H8, Piece::King, Color::Black).unwrap();
        board.set_turn(Color::Black);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        println!("Testing board:\n{}", board);

        let expected_moves = vec![
            std_move!(E7, E1),
            std_move!(A1, E1, (Piece::Queen, Color::Black)),
            std_move!(E8, E1, (Piece::Rook, Color::White)),
        ];
        let mut expected_move_iter = expected_moves.iter();

        let move1 = searcher.search(&mut board, &mut move_generator).unwrap();
        move1.apply(&mut board).unwrap();
        board.toggle_turn();
        assert_eq!(
            expected_move_iter.next().unwrap(),
            &move1,
            "failed to find first move of mate in 2"
        );
        println!("Testing board:\n{}", board);

        let move2 = searcher.search(&mut board, &mut move_generator).unwrap();
        move2.apply(&mut board).unwrap();
        board.toggle_turn();
        assert_eq!(
            expected_move_iter.next().unwrap(),
            &move2,
            "failed to find second move of mate in 2"
        );
        println!("Testing board:\n{}", board);

        let move3 = searcher.search(&mut board, &mut move_generator).unwrap();
        move3.apply(&mut board).unwrap();
        board.toggle_turn();
        assert_eq!(
            expected_move_iter.next().unwrap(),
            &move3,
            "failed to find third move of mate in 2"
        );
        println!("Testing board:\n{}", board);
    }
}
