use crate::board::Board;
use crate::chess_move::ChessMove;
use crate::evaluate;
use crate::move_generator::MoveGenerator;
use log::debug;
use rustc_hash::FxHashMap;
use thiserror::Error;

type SearchNode = (u64, i16, i16); // position_hash, alpha, beta
type SearchResult = i16; // best_score

pub struct AlphaBetaSearcher {
    search_depth: u8,
    search_result_cache: FxHashMap<SearchNode, SearchResult>,
    searched_position_count: usize,
    cache_hit_count: usize,
    termination_count: usize,
    current_line_stack: Vec<ChessMove>,
}

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("no available moves")]
    NoAvailableMoves,
}

impl AlphaBetaSearcher {
    pub fn new(depth: u8) -> Self {
        Self {
            search_depth: depth,
            search_result_cache: FxHashMap::default(),
            searched_position_count: 0,
            cache_hit_count: 0,
            termination_count: 0,
            current_line_stack: Vec::new(),
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

    pub fn reset_stats(&mut self) {
        self.searched_position_count = 0;
        self.cache_hit_count = 0;
        self.termination_count = 0;
    }

    pub fn search(
        &mut self,
        board: &mut Board,
        move_generator: &mut MoveGenerator,
    ) -> Result<ChessMove, SearchError> {
        self.reset_stats();

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
                self.push_move_to_current_line(chess_move.clone());
                debug!("Current line: {:?}", self.current_line_stack);

                let score = self.alpha_beta_max(
                    self.search_depth,
                    board,
                    move_generator,
                    i16::MIN,
                    i16::MAX,
                );

                chess_move.undo(board).unwrap();
                board.toggle_turn();
                self.pop_move_from_current_line();

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
            debug!("Eval score (max): {}", score);
            return score;
        }

        if let Some(cached_score) = self.check_cache(board.current_position_hash(), alpha, beta) {
            return cached_score;
        }
        let candidates = move_generator.generate_moves(board, board.turn());

        for chess_move in candidates.iter() {
            chess_move.apply(board).unwrap();
            board.toggle_turn();
            self.push_move_to_current_line(chess_move.clone());
            debug!("Current line: {:?}", self.current_line_stack);

            let score = self.alpha_beta_min(depth - 1, board, move_generator, alpha, beta);
            let board_hash = board.current_position_hash();

            chess_move.undo(board).unwrap();
            board.toggle_turn();
            self.pop_move_from_current_line();

            if score >= beta {
                self.termination_count += 1;
                self.set_cache(board_hash, alpha, beta, score);
                return beta;
            }

            if score > alpha {
                alpha = score;
            }
        }

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
            let eval_score = evaluate::score(board, move_generator, board.turn());
            debug!("Eval score (min): {}", eval_score);
            let score = -eval_score;
            return score;
        }

        if let Some(cached_score) = self.check_cache(board.current_position_hash(), alpha, beta) {
            return cached_score;
        }
        let candidates = move_generator.generate_moves(board, board.turn());

        for chess_move in candidates.iter() {
            chess_move.apply(board).unwrap();
            board.toggle_turn();
            self.push_move_to_current_line(chess_move.clone());
            debug!("Current line: {:?}", self.current_line_stack);

            let score = self.alpha_beta_max(depth - 1, board, move_generator, alpha, beta);
            let board_hash = board.current_position_hash();

            chess_move.undo(board).unwrap();
            board.toggle_turn();
            self.pop_move_from_current_line();

            if score <= alpha {
                self.termination_count += 1;
                self.set_cache(board_hash, alpha, beta, score);
                return alpha;
            }

            if score < beta {
                beta = score;
            }
        }

        beta
    }

    fn set_cache(&mut self, position_hash: u64, alpha: i16, beta: i16, score: i16) {
        let search_node = (position_hash, alpha, beta);
        self.search_result_cache.insert(search_node, score);
    }

    fn check_cache(&mut self, position_hash: u64, alpha: i16, beta: i16) -> Option<i16> {
        let search_node = (position_hash, alpha, beta);
        match self.search_result_cache.get(&search_node) {
            Some(&prev_best_score) => {
                self.cache_hit_count += 1;
                Some(prev_best_score)
            }
            None => None,
        }
    }

    fn push_move_to_current_line(&mut self, chess_move: ChessMove) {
        self.current_line_stack.push(chess_move);
    }

    fn pop_move_from_current_line(&mut self) {
        self.current_line_stack.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::castle_rights::ALL_CASTLE_RIGHTS;
    use crate::board::color::Color;
    use crate::board::piece::Piece;
    use crate::chess_move::standard::StandardChessMove;
    use crate::{chess_position, std_move};
    use common::bitboard::bitboard::Bitboard;
    use common::bitboard::square::*;

    #[test]
    fn test_find_mate_in_1_white() {
        let mut searcher = AlphaBetaSearcher::new(1);
        let mut move_generator = MoveGenerator::new();

        let mut board = chess_position! {
            .Q......
            ........
            ........
            ........
            ........
            ........
            k.K.....
            ........
        };
        board.set_turn(Color::White);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);
        println!("Testing board:\n{}", board);

        let chess_move = searcher.search(&mut board, &mut move_generator).unwrap();
        let valid_checkmates = [std_move!(B8, B2), std_move!(B8, A8), std_move!(B8, A7)];
        assert!(
            valid_checkmates.contains(&chess_move),
            "{} does not leed to checkmate",
            chess_move
        );
    }

    #[test]
    fn test_find_mate_in_1_black() {
        let mut searcher = AlphaBetaSearcher::new(1);
        let mut move_generator = MoveGenerator::new();
        let mut board = chess_position! {
            .q......
            ........
            ........
            ........
            ........
            ........
            K.k.....
            ........
        };
        board.set_turn(Color::Black);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        println!("Testing board:\n{}", board);

        let chess_move = searcher.search(&mut board, &mut move_generator).unwrap();

        let valid_checkmates = [std_move!(B8, B2), std_move!(B8, A8), std_move!(B8, A7)];
        assert!(valid_checkmates.contains(&chess_move));
    }

    #[test]
    fn test_find_back_rank_mate_in_2_white() {
        let mut searcher = AlphaBetaSearcher::new(2);
        let mut move_generator = MoveGenerator::new();

        let mut board = chess_position! {
            .k.....r
            ppp.....
            ........
            ........
            ........
            ........
            ...Q....
            K..R....
        };
        board.set_turn(Color::White);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        println!("Testing board:\n{}", board);

        let expected_moves = [
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
        let mut searcher = AlphaBetaSearcher::new(3);
        let mut move_generator = MoveGenerator::new();

        let mut board = chess_position! {
            ....r..k
            ....q...
            ........
            ........
            ........
            ........
            .....PPP
            R.....K.
        };
        board.set_turn(Color::Black);
        board.lose_castle_rights(ALL_CASTLE_RIGHTS);

        println!("Testing board:\n{}", board);

        let expected_moves = [
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
