use crate::board::Board;
use crate::chess_move::chess_move::ChessMove;
use crate::evaluate;
use crate::move_generator::MoveGenerator;
use log::{debug, trace};
use rustc_hash::FxHashMap;
use thiserror::Error;

use rayon::prelude::*;
use std::cmp::{max, min};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use self::prioritize_chess_moves::sort_chess_moves;

type SearchNode = (u64, i16, i16); // position_hash, alpha, beta
type SearchResult = i16; // best_score

mod prioritize_chess_moves;

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
    last_score: Option<i16>,
    last_search_duration: Option<Duration>,
}

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("no available moves")]
    NoAvailableMoves,
    #[error("depth must be at least 1")]
    DepthTooLow,
}

impl SearchContext {
    pub fn new(depth: u8) -> Self {
        Self {
            search_depth: depth,
            search_result_cache: Arc::new(RwLock::new(FxHashMap::default())),
            searched_position_count: Arc::new(RwLock::new(0)),
            cache_hit_count: Arc::new(RwLock::new(0)),
            termination_count: Arc::new(RwLock::new(0)),
            last_score: None,
            last_search_duration: None,
        }
    }

    pub fn reset_stats(&mut self) {
        self.last_score = None;
        self.last_search_duration = None;
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

    pub fn last_score(&self) -> Option<i16> {
        self.last_score
    }

    pub fn last_search_duration(&self) -> Option<Duration> {
        self.last_search_duration
    }
}

pub fn alpha_beta_search(
    context: &mut SearchContext,
    board: &mut Board,
    move_generator: &mut MoveGenerator,
) -> Result<ChessMove, SearchError> {
    context.reset_stats();
    debug!("alpha-beta search depth: {}", context.search_depth());

    if context.search_depth() < 1 {
        return Err(SearchError::DepthTooLow);
    }

    let start = Instant::now();
    let current_player = board.turn();
    let current_player_is_maximizing = current_player.maximize_score();
    let mut candidates =
        move_generator.generate_moves_and_lazily_update_chess_move_effects(board, current_player);
    sort_chess_moves(&mut candidates, &board);

    // First, score each of the candidates. Note: `par_iter` is a rayon
    // primitive that allows for parallel iteration over a collection.
    let scored_moves = candidates.par_iter().map(|chess_move| {
        let mut local_board = board.clone();
        let mut local_move_generator = MoveGenerator::new();
        let mut local_context = context.clone();
        let local_depth = context.search_depth();

        chess_move.apply(&mut local_board).unwrap();
        local_board.toggle_turn();

        let score = alpha_beta_minimax(
            &mut local_context,
            &mut local_board,
            &mut local_move_generator,
            local_depth - 1,
            i16::MIN,
            i16::MAX,
            // The current iteration is for `current_player_is_maximizing == true`,
            // so the next layer of alpha-beta should do the opposite.
            !current_player_is_maximizing,
        )
        .unwrap();

        chess_move.undo(&mut local_board).unwrap();
        local_board.toggle_turn();

        (score, chess_move.clone())
    });

    // Sort the best move to the end so we can pop it off.
    let mut scored_moves = scored_moves.collect::<Vec<_>>();
    scored_moves.sort_by(|(a, _), (b, _)| b.cmp(a));
    debug!(
        "Alpha-beta search results before sorting: {:?}",
        scored_moves
    );
    if current_player_is_maximizing {
        scored_moves.reverse();
    }
    debug!(
        "Alpha-beta search results after sorting: {:?}",
        scored_moves
    );

    let (score, result) = scored_moves.pop().unwrap();
    context.last_score = Some(score);
    context.last_search_duration = Some(start.elapsed());
    debug!(
        "Alpha-beta search returning best move: {:?} (score: {})",
        result, score
    );
    Ok(result)
}

fn alpha_beta_minimax(
    context: &mut SearchContext,
    board: &mut Board,
    move_generator: &mut MoveGenerator,
    depth: u8,
    alpha: i16,
    beta: i16,
    maximizing_player: bool,
) -> Result<i16, SearchError> {
    let search_node = (board.current_position_hash(), alpha, beta);
    if let Some(score) = check_cache(context, search_node) {
        trace!(
            "{}alpha_beta_minimax returning cached score: {} for depth: {}",
            "  ".repeat((context.search_depth() - depth) as usize),
            score,
            depth
        );
        return Ok(score);
    }

    trace!(
        "{}alpha_beta_minimax(depth: {}, alpha: {}, beta: {}, maximizing_player: {})",
        "  ".repeat((context.search_depth() - depth) as usize),
        depth,
        alpha,
        beta,
        maximizing_player
    );

    {
        let mut count = context.searched_position_count.write().unwrap();
        *count += 1;
    }

    let current_turn = board.turn();
    if depth == 0 {
        let score = evaluate::score(board, move_generator, current_turn, depth);
        trace!(
            "{}alpha_beta_minimax returning score (terminal depth): {} for depth: {}",
            "  ".repeat((context.search_depth() - depth) as usize),
            score,
            depth
        );
        set_cache(context, search_node, score);
        return Ok(score);
    }

    let mut candidates =
        move_generator.generate_moves_and_lazily_update_chess_move_effects(board, current_turn);
    sort_chess_moves(&mut candidates, &board);

    if candidates.is_empty() {
        let score = evaluate::score(board, move_generator, current_turn, depth);
        trace!(
            "{}alpha_beta_minimax returning score (no moves): {} for depth: {}",
            "  ".repeat((context.search_depth() - depth) as usize),
            score,
            depth
        );
        set_cache(context, search_node, score);
        return Ok(score);
    }

    if maximizing_player {
        let mut value = std::i16::MIN;
        let mut alpha = alpha;
        for chess_move in candidates.iter() {
            chess_move.apply(board).unwrap();
            board.toggle_turn();
            value = max(
                value,
                alpha_beta_minimax(
                    context,
                    board,
                    move_generator,
                    depth - 1,
                    alpha,
                    beta,
                    false,
                )
                .unwrap(),
            );
            chess_move.undo(board).unwrap();
            board.toggle_turn();

            alpha = max(alpha, value);
            if beta <= alpha {
                break;
            }
        }
        set_cache(context, search_node, value);
        Ok(value)
    } else {
        let mut value = std::i16::MAX;
        let mut beta = beta;
        for chess_move in candidates.iter() {
            chess_move.apply(board).unwrap();
            board.toggle_turn();
            value = min(
                value,
                alpha_beta_minimax(context, board, move_generator, depth - 1, alpha, beta, true)
                    .unwrap(),
            );
            chess_move.undo(board).unwrap();
            board.toggle_turn();

            beta = min(beta, value);
            if beta <= alpha {
                break;
            }
        }
        set_cache(context, search_node, value);
        Ok(value)
    }
}

fn set_cache(context: &mut SearchContext, search_node: SearchNode, score: i16) {
    let mut cache = context.search_result_cache.write().unwrap();
    cache.insert(search_node, score);
}

fn check_cache(context: &mut SearchContext, search_node: SearchNode) -> Option<i16> {
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
    use crate::chess_move::chess_move_effect::ChessMoveEffect;
    use crate::chess_move::standard::StandardChessMove;
    use crate::{check_move, checkmate_move, chess_position, std_move};
    use common::bitboard::bitboard::Bitboard;
    use common::bitboard::square::*;

    #[test]
    fn test_find_mate_in_1_white() {
        let mut search_context = SearchContext::new(4);
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
        let valid_checkmates = [
            checkmate_move!(std_move!(B8, B2)),
            checkmate_move!(std_move!(B8, A8)),
            checkmate_move!(std_move!(B8, A7)),
        ];
        assert!(
            valid_checkmates.contains(&chess_move),
            "{} does not lead to checkmate",
            chess_move
        );
    }

    #[test]
    fn test_find_mate_in_1_black() {
        let mut search_context = SearchContext::new(4);
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

        let valid_checkmates = [
            checkmate_move!(std_move!(B8, B2)),
            checkmate_move!(std_move!(B8, A8)),
            checkmate_move!(std_move!(B8, A7)),
        ];
        assert!(
            valid_checkmates.contains(&chess_move),
            "{} does not lead to checkmate",
            chess_move
        );
    }

    #[test]
    fn test_find_back_rank_mate_in_2_white() {
        let mut search_context = SearchContext::new(4);
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
            check_move!(std_move!(D2, D8)),
            std_move!(H8, D8, Capture(Piece::Queen)),
            checkmate_move!(std_move!(D1, D8, Capture(Piece::Rook))),
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
        let mut search_context = SearchContext::new(4);
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
            check_move!(std_move!(E7, E1)),
            std_move!(A1, E1, Capture(Piece::Queen)),
            checkmate_move!(std_move!(E8, E1, Capture(Piece::Rook))),
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
