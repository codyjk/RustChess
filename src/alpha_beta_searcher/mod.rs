//! Generic alpha-beta search algorithm.

use log::debug;
use rayon::prelude::*;
use std::cmp::{max, min};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use thiserror::Error;

mod traits;
mod transposition_table;

pub use traits::*;
pub use transposition_table::{BoundType, TranspositionTable};

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("no available moves")]
    NoAvailableMoves,
    #[error("depth must be at least 1")]
    DepthTooLow,
}

pub struct SearchContext<M: Clone + Send + Sync> {
    search_depth: u8,
    searched_position_count: AtomicUsize,
    last_score: Option<i16>,
    last_search_duration: Option<Duration>,
    transposition_table: TranspositionTable<M>,
}

impl<M: Clone + Send + Sync> SearchContext<M> {
    pub fn new(depth: u8) -> Self {
        Self {
            search_depth: depth,
            searched_position_count: AtomicUsize::new(0),
            last_score: None,
            last_search_duration: None,
            transposition_table: TranspositionTable::default(),
        }
    }

    pub fn reset_stats(&mut self) {
        self.last_score = None;
        self.last_search_duration = None;
        self.searched_position_count.store(0, Ordering::SeqCst);
        self.transposition_table.clear();
    }

    pub fn searched_position_count(&self) -> usize {
        self.searched_position_count.load(Ordering::SeqCst)
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

    fn increment_position_count(&self) {
        self.searched_position_count.fetch_add(1, Ordering::SeqCst);
    }
}

#[must_use = "search returns the best move found"]
pub fn alpha_beta_search<S, G, E, O>(
    context: &mut SearchContext<G::Move>,
    state: &mut S,
    move_generator: &G,
    evaluator: &E,
    move_orderer: &O,
) -> Result<G::Move, SearchError>
where
    S: GameState,
    G: MoveGenerator<S>,
    G::Move: GameMove<State = S>,
    E: Evaluator<S>,
    O: MoveOrderer<S, G::Move>,
{
    debug!("alpha-beta search depth: {}", context.search_depth());
    let depth = context.search_depth();

    if depth < 1 {
        return Err(SearchError::DepthTooLow);
    }

    let start = Instant::now();
    let current_player_is_maximizing = state.is_maximizing_player();
    let mut candidates = move_generator.generate_moves(state);

    move_orderer.order_moves(candidates.as_mut(), state);

    let hash = state.position_hash();
    if let Some((score, best_move)) =
        context
            .transposition_table
            .probe(hash, depth, i16::MIN, i16::MAX)
    {
        if let Some(ref mv) = best_move {
            if candidates.as_ref().iter().any(|c| c == mv) {
                debug!("Using transposition table hit");
                context.last_score = Some(score);
                context.last_search_duration = Some(start.elapsed());
                return Ok(mv.clone());
            }
        }
    }

    let scored_moves: Vec<_> = candidates
        .as_ref()
        .par_iter()
        .map(|game_move| {
            let mut local_state = state.clone();
            let local_generator = move_generator.clone();
            let local_evaluator = evaluator.clone();
            let local_orderer = move_orderer.clone();

            game_move
                .apply(&mut local_state)
                .expect("move application should succeed in search");
            local_state.toggle_turn();

        let score = alpha_beta_minimax(
            context,
                &mut local_state,
                &local_generator,
                &local_evaluator,
                &local_orderer,
            depth - 1,
            i16::MIN,
            i16::MAX,
            !current_player_is_maximizing,
        )
        .unwrap();

            (score, game_move.clone())
        })
        .collect();

    let mut scored_moves = scored_moves;
    scored_moves.sort_by(|(a, _), (b, _)| b.cmp(a));
    if current_player_is_maximizing {
        scored_moves.reverse();
    }

    let (score, best_move) = scored_moves.pop().ok_or(SearchError::NoAvailableMoves)?;

    context.transposition_table.store(
        hash,
        score,
        depth,
        BoundType::Exact,
        Some(best_move.clone()),
    );

    context.last_score = Some(score);
    context.last_search_duration = Some(start.elapsed());

    Ok(best_move)
}

fn alpha_beta_minimax<S, G, E, O>(
    context: &SearchContext<G::Move>,
    state: &mut S,
    move_generator: &G,
    evaluator: &E,
    move_orderer: &O,
    depth: u8,
    mut alpha: i16,
    mut beta: i16,
    maximizing_player: bool,
) -> Result<i16, SearchError>
where
    S: GameState,
    G: MoveGenerator<S>,
    G::Move: GameMove<State = S>,
    E: Evaluator<S>,
    O: MoveOrderer<S, G::Move>,
{
    context.increment_position_count();

    let hash = state.position_hash();

    if let Some((score, _)) = context.transposition_table.probe(hash, depth, alpha, beta) {
        return Ok(score);
    }

    if depth == 0 {
        let score = evaluator.evaluate(state, depth);
        return Ok(score);
    }

    let mut candidates = move_generator.generate_moves(state);

    if candidates.is_empty() {
        let score = evaluator.evaluate(state, depth);
        return Ok(score);
    }

    move_orderer.order_moves(candidates.as_mut(), state);

    let mut best_move = None;
    let mut best_score = if maximizing_player {
        i16::MIN
    } else {
        i16::MAX
    };
    let original_alpha = alpha;

    for game_move in candidates.as_ref().iter() {
        game_move
            .apply(state)
            .expect("move application should succeed in search");
        state.toggle_turn();

        let score = alpha_beta_minimax(
            context,
            state,
            move_generator,
            evaluator,
            move_orderer,
            depth - 1,
            alpha,
            beta,
            !maximizing_player,
        )?;

        game_move
            .undo(state)
            .expect("move undo should succeed in search");
        state.toggle_turn();

        if maximizing_player {
            if score > best_score {
                best_score = score;
                best_move = Some(game_move.clone());
            }
            alpha = max(alpha, score);
        } else {
            if score < best_score {
                best_score = score;
                best_move = Some(game_move.clone());
            }
            beta = min(beta, score);
        }

        if beta <= alpha {
            break;
        }
    }

    let bound_type = if best_score <= original_alpha {
        BoundType::Upper
    } else if best_score >= beta {
        BoundType::Lower
    } else {
        BoundType::Exact
    };

    context
        .transposition_table
        .store(hash, best_score, depth, bound_type, best_move);

    Ok(best_score)
}

#[cfg(test)]
mod tests;
