use crate::board::error::BoardError;
use crate::board::Board;
use crate::chess_move::chess_move::ChessMove;
use crate::evaluate;
use crate::move_generator::MoveGenerator;
use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use thiserror::Error;

use rayon::prelude::*;
use std::sync::{Arc, RwLock};

type SearchNode = (u64, i16, i16); // position_hash, alpha, beta
type SearchResult = i16; // best_score

/// Represents the state and control of a search for the best move in a chess position.
/// The search is implemented using alpha-beta minimax search, and uses `rayon`
/// to parallelize the search across multiple threads. Access to the search context is thread-safe.
#[derive(Clone)]
pub struct SearchContext {
    search_depth: u8,
    search_result_cache: Arc<RwLock<FxHashMap<SearchNode, SearchResult>>>,
    searched_position_count: Arc<RwLock<usize>>,
    cache_hit_count: Arc<RwLock<usize>>,
    termination_count: Arc<RwLock<usize>>,
}

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("no available moves")]
    NoAvailableMoves,
}

use log::Level;
use std::cell::RefCell;
use std::thread_local;

thread_local! {
    static CURRENT_LINE: RefCell<SmallVec<[ChessMove; 10]>> = RefCell::new(SmallVec::new());
}

struct ThreadLocalCurrentLine;

impl ThreadLocalCurrentLine {
    #[inline(always)]
    fn push(chess_move: ChessMove) {
        if log::log_enabled!(Level::Trace) {
            CURRENT_LINE.with(|line| line.borrow_mut().push(chess_move));
        }
    }

    #[inline(always)]
    fn pop() {
        if log::log_enabled!(Level::Trace) {
            CURRENT_LINE.with(|line| {
                line.borrow_mut().pop();
            });
        }
    }

    #[inline(always)]
    fn get() -> Option<SmallVec<[ChessMove; 10]>> {
        if log::log_enabled!(Level::Trace) {
            Some(CURRENT_LINE.with(|line| line.borrow().clone()))
        } else {
            None
        }
    }
}

use log::{error, trace};

#[inline(always)]
fn trace_push_move(chess_move: &ChessMove, depth: u8, search_depth: u8) {
    if log::log_enabled!(Level::Trace) {
        trace!(
            "{:indent$}Evaluating move: {} at depth {}",
            "",
            chess_move,
            depth,
            indent = (search_depth - depth) as usize * 2
        );
        ThreadLocalCurrentLine::push(chess_move.clone());
    }
}

#[inline(always)]
fn trace_pop_move() {
    if log::log_enabled!(Level::Trace) {
        ThreadLocalCurrentLine::pop();
    }
}

#[inline(always)]
fn trace_error(e: &BoardError) {
    if log::log_enabled!(Level::Trace) {
        error!("Error applying move: {:?}", e);
        if let Some(current_line) = ThreadLocalCurrentLine::get() {
            error!("Current move history: {:?}", current_line);
        }
    }
}

impl SearchContext {
    pub fn new(depth: u8) -> Self {
        Self {
            search_depth: depth,
            search_result_cache: Arc::new(RwLock::new(FxHashMap::default())),
            searched_position_count: Arc::new(RwLock::new(0)),
            cache_hit_count: Arc::new(RwLock::new(0)),
            termination_count: Arc::new(RwLock::new(0)),
        }
    }

    pub fn reset_stats(&mut self) {
        *self.searched_position_count.write().unwrap() = 0;
        *self.cache_hit_count.write().unwrap() = 0;
        *self.termination_count.write().unwrap() = 0;
    }

    pub fn searched_position_count(&self) -> usize {
        *self.searched_position_count.read().unwrap()
    }

    pub fn cache_hit_count(&self) -> usize {
        *self.cache_hit_count.read().unwrap()
    }

    pub fn termination_count(&self) -> usize {
        *self.termination_count.read().unwrap()
    }

    pub fn search_depth(&self) -> u8 {
        self.search_depth
    }
}

/// Implements alpha-beta minimax search to find the "best" move to a given depth.
/// The "best" move is determined by the scoring function implemented in the `evaluate` module.
pub fn alpha_beta_search(
    context: &mut SearchContext,
    board: &mut Board,
    move_generator: &mut MoveGenerator,
) -> Result<ChessMove, SearchError> {
    context.reset_stats();

    let current_turn = board.turn();
    let candidates = move_generator.generate_moves(board, current_turn);

    if candidates.is_empty() {
        return Err(SearchError::NoAvailableMoves);
    }

    // `par_iter` is a rayon primitive that allows for parallel iteration over a collection.
    let scores: Vec<(ChessMove, i16)> = candidates
        .par_iter()
        .map(|chess_move| {
            let mut local_board = board.clone();
            let mut local_move_generator = MoveGenerator::new();
            let mut local_context = context.clone();
            let search_depth = context.search_depth;

            trace_push_move(chess_move, search_depth, search_depth);

            chess_move.apply(&mut local_board).unwrap();
            local_board.toggle_turn();

            let score = alpha_beta_max(
                &mut local_context,
                search_depth,
                &mut local_board,
                &mut local_move_generator,
                i16::MIN,
                i16::MAX,
            );

            trace_pop_move();

            chess_move.undo(&mut local_board).unwrap();
            local_board.toggle_turn();

            (chess_move.clone(), score)
        })
        .collect();

    let mut results = scores;
    results.sort_by(|(_, score_a), (_, score_b)| score_a.partial_cmp(score_b).unwrap());
    results.reverse();
    let (best_move, _) = results.pop().unwrap();
    Ok(best_move)
}

fn alpha_beta_max(
    context: &mut SearchContext,
    depth: u8,
    board: &mut Board,
    move_generator: &mut MoveGenerator,
    mut alpha: i16,
    beta: i16,
) -> i16 {
    {
        let mut count = context.searched_position_count.write().unwrap();
        *count += 1;
    }

    if depth == 0 {
        return evaluate::score(board, move_generator, board.turn());
    }

    if let Some(cached_score) = check_cache(context, board.current_position_hash(), alpha, beta) {
        return cached_score;
    }

    let candidates = move_generator.generate_moves(board, board.turn());

    for chess_move in candidates.iter() {
        trace_push_move(chess_move, depth, context.search_depth);

        chess_move
            .apply(board)
            .map_err(|e| {
                trace_error(&e);
                e
            })
            .unwrap();
        board.toggle_turn();

        let score = alpha_beta_min(context, depth - 1, board, move_generator, alpha, beta);
        let board_hash = board.current_position_hash();

        chess_move.undo(board).unwrap();
        board.toggle_turn();

        if score >= beta {
            {
                let mut count = context.termination_count.write().unwrap();
                *count += 1;
            }
            set_cache(context, board_hash, alpha, beta, score);
            return beta;
        }

        if score > alpha {
            alpha = score;
        }

        trace_pop_move();
    }

    alpha
}

fn alpha_beta_min(
    context: &mut SearchContext,
    depth: u8,
    board: &mut Board,
    move_generator: &mut MoveGenerator,
    alpha: i16,
    mut beta: i16,
) -> i16 {
    {
        let mut count = context.searched_position_count.write().unwrap();
        *count += 1;
    }

    if depth == 0 {
        return -evaluate::score(board, move_generator, board.turn());
    }

    if let Some(cached_score) = check_cache(context, board.current_position_hash(), alpha, beta) {
        return cached_score;
    }

    let candidates = move_generator.generate_moves(board, board.turn());

    for chess_move in candidates.iter() {
        trace_push_move(chess_move, depth, context.search_depth);

        chess_move
            .apply(board)
            .map_err(|e| {
                trace_error(&e);
                e
            })
            .unwrap();
        board.toggle_turn();

        let score = alpha_beta_max(context, depth - 1, board, move_generator, alpha, beta);
        let board_hash = board.current_position_hash();

        chess_move.undo(board).unwrap();
        board.toggle_turn();

        if score <= alpha {
            {
                let mut count = context.termination_count.write().unwrap();
                *count += 1;
            }
            set_cache(context, board_hash, alpha, beta, score);
            return alpha;
        }

        if score < beta {
            beta = score;
        }

        trace_pop_move();
    }

    beta
}

fn set_cache(context: &mut SearchContext, position_hash: u64, alpha: i16, beta: i16, score: i16) {
    let search_node = (position_hash, alpha, beta);
    let mut cache = context.search_result_cache.write().unwrap();
    cache.insert(search_node, score);
}

fn check_cache(
    context: &mut SearchContext,
    position_hash: u64,
    alpha: i16,
    beta: i16,
) -> Option<i16> {
    let search_node = (position_hash, alpha, beta);
    let cache = context.search_result_cache.read().unwrap();
    match cache.get(&search_node) {
        Some(&prev_best_score) => {
            let mut count = context.cache_hit_count.write().unwrap();
            *count += 1;
            Some(prev_best_score)
        }
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::castle_rights_bitmask::ALL_CASTLE_RIGHTS;
    use crate::board::color::Color;
    use crate::board::piece::Piece;
    use crate::chess_move::capture::Capture;
    use crate::chess_move::standard::StandardChessMove;
    use crate::{chess_position, std_move};
    use common::bitboard::bitboard::Bitboard;
    use common::bitboard::square::*;

    #[test]
    fn test_find_mate_in_1_white() {
        let mut search_context = SearchContext::new(1);
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

        let chess_move =
            alpha_beta_search(&mut search_context, &mut board, &mut move_generator).unwrap();
        let valid_checkmates = [std_move!(B8, B2), std_move!(B8, A8), std_move!(B8, A7)];
        assert!(
            valid_checkmates.contains(&chess_move),
            "{} does not leed to checkmate",
            chess_move
        );
    }

    #[test]
    fn test_find_mate_in_1_black() {
        let mut search_context = SearchContext::new(1);
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

        let chess_move =
            alpha_beta_search(&mut search_context, &mut board, &mut move_generator).unwrap();

        let valid_checkmates = [std_move!(B8, B2), std_move!(B8, A8), std_move!(B8, A7)];
        assert!(valid_checkmates.contains(&chess_move));
    }

    #[test]
    fn test_find_back_rank_mate_in_2_white() {
        let mut search_context = SearchContext::new(3);
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
            std_move!(H8, D8, Capture(Piece::Queen)),
            std_move!(D1, D8, Capture(Piece::Rook)),
        ];
        let mut expected_move_iter = expected_moves.iter();

        let move1 =
            alpha_beta_search(&mut search_context, &mut board, &mut move_generator).unwrap();
        move1.apply(&mut board).unwrap();
        board.toggle_turn();
        assert_eq!(expected_move_iter.next().unwrap(), &move1);
        println!("Testing board:\n{}", board);

        let move2 =
            alpha_beta_search(&mut search_context, &mut board, &mut move_generator).unwrap();
        move2.apply(&mut board).unwrap();
        board.toggle_turn();
        assert_eq!(expected_move_iter.next().unwrap(), &move2);
        println!("Testing board:\n{}", board);

        let move3 =
            alpha_beta_search(&mut search_context, &mut board, &mut move_generator).unwrap();
        move3.apply(&mut board).unwrap();
        board.toggle_turn();
        assert_eq!(expected_move_iter.next().unwrap(), &move3);
        println!("Testing board:\n{}", board);
    }

    #[test]
    fn test_find_back_rank_mate_in_2_black() {
        let mut search_context = SearchContext::new(3);
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
            std_move!(A1, E1, Capture(Piece::Queen)),
            std_move!(E8, E1, Capture(Piece::Rook)),
        ];
        let mut expected_move_iter = expected_moves.iter();

        let move1 =
            alpha_beta_search(&mut search_context, &mut board, &mut move_generator).unwrap();
        move1.apply(&mut board).unwrap();
        board.toggle_turn();
        assert_eq!(
            expected_move_iter.next().unwrap(),
            &move1,
            "failed to find first move of mate in 2"
        );
        println!("Testing board:\n{}", board);

        let move2 =
            alpha_beta_search(&mut search_context, &mut board, &mut move_generator).unwrap();
        move2.apply(&mut board).unwrap();
        board.toggle_turn();
        assert_eq!(
            expected_move_iter.next().unwrap(),
            &move2,
            "failed to find second move of mate in 2"
        );
        println!("Testing board:\n{}", board);

        let move3 =
            alpha_beta_search(&mut search_context, &mut board, &mut move_generator).unwrap();
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
