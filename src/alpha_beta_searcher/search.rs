//! Alpha-beta search algorithm implementation.
//!
//! Optimizations:
//! - Thread-local storage for killer moves to eliminate lock contention in parallel search
//! - Transposition tables for position caching
//! - Move ordering heuristics (PV moves, killer moves, captures)
//! - Quiescence search: continues searching tactical moves (captures, checks) beyond the
//!   nominal depth limit to avoid horizon effects. Games opt-in by implementing `is_tactical`
//!   on their move type.
//! - Null move pruning: gives opponent a free move; if they still can't beat beta, prunes the
//!   branch. Games opt-in by implementing `is_in_check` and `is_endgame` on their state type.
//! - Iterative deepening: searches at increasing depths (1..target), using previous results to
//!   improve move ordering. The best move from depth N-1 is prioritized at depth N, improving
//!   alpha-beta pruning efficiency.

use std::cmp::{max, min};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use log::debug;
use rayon::prelude::*;
use thiserror::Error;

use super::killer_moves::KillerMovesManager;
use super::transposition_table::{BoundType, TranspositionTable};
use super::{Evaluator, GameMove, GameState, MoveCollection, MoveGenerator, MoveOrderer};

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("no available moves")]
    NoAvailableMoves,
    #[error("depth must be at least 1")]
    DepthTooLow,
}

/// Search configuration parameters.
struct SearchConfig {
    depth: u8,
    parallel: bool,
}

impl SearchConfig {
    fn new(depth: u8, parallel: bool) -> Self {
        Self { depth, parallel }
    }
}

/// Statistics collected during search.
struct SearchStats {
    position_count: AtomicUsize,
    last_score: Option<i16>,
    last_duration: Option<Duration>,
}

impl SearchStats {
    fn new() -> Self {
        Self {
            position_count: AtomicUsize::new(0),
            last_score: None,
            last_duration: None,
        }
    }

    fn increment(&self) {
        self.position_count.fetch_add(1, Ordering::SeqCst);
    }

    fn reset(&mut self) {
        self.last_score = None;
        self.last_duration = None;
        self.position_count.store(0, Ordering::SeqCst);
    }

    fn record_result(&mut self, score: i16, duration: Duration) {
        self.last_score = Some(score);
        self.last_duration = Some(duration);
    }

    fn count(&self) -> usize {
        self.position_count.load(Ordering::SeqCst)
    }
}

pub struct SearchContext<M: Clone + Send + Sync + 'static> {
    config: SearchConfig,
    stats: SearchStats,
    transposition_table: TranspositionTable<M>,
    killer_manager: KillerMovesManager,
}

impl<M: Clone + Send + Sync + 'static> SearchContext<M> {
    pub fn new(depth: u8) -> Self {
        Self {
            config: SearchConfig::new(depth, true),
            stats: SearchStats::new(),
            transposition_table: TranspositionTable::default(),
            killer_manager: KillerMovesManager::new(depth),
        }
    }

    pub fn with_parallel(depth: u8, parallel: bool) -> Self {
        Self {
            config: SearchConfig::new(depth, parallel),
            stats: SearchStats::new(),
            transposition_table: TranspositionTable::default(),
            killer_manager: KillerMovesManager::new(depth),
        }
    }

    pub fn set_parallel(&mut self, parallel: bool) {
        self.config.parallel = parallel;
    }

    pub fn is_parallel(&self) -> bool {
        self.config.parallel
    }

    pub fn reset_stats(&mut self) {
        self.stats.reset();
        self.transposition_table.clear();
        self.killer_manager.clear();
    }

    pub fn store_killer(&self, ply: u8, killer: M) {
        self.killer_manager.store(ply, killer);
    }

    pub fn get_killers(&self, ply: u8) -> [Option<M>; 2] {
        self.killer_manager.get(ply)
    }

    pub fn clear_killers(&mut self) {
        self.killer_manager.clear();
    }

    pub fn searched_position_count(&self) -> usize {
        self.stats.count()
    }

    pub fn search_depth(&self) -> u8 {
        self.config.depth
    }

    pub fn last_score(&self) -> Option<i16> {
        self.stats.last_score
    }

    pub fn last_search_duration(&self) -> Option<Duration> {
        self.stats.last_duration
    }

    pub fn tt_hits(&self) -> usize {
        self.transposition_table.hits()
    }

    fn increment_position_count(&self) {
        self.stats.increment();
    }
}

/// Applies a move, executes a closure with the new state, then undoes the move.
/// Handles turn toggling automatically.
fn with_move_applied<S, M, F, R>(game_move: &M, state: &mut S, f: F) -> Result<R, SearchError>
where
    S: GameState,
    M: GameMove<State = S>,
    F: FnOnce(&mut S) -> Result<R, SearchError>,
{
    game_move
        .apply(state)
        .expect("move application should succeed in search");
    state.toggle_turn();

    let result = f(state);

    game_move
        .undo(state)
        .expect("move undo should succeed in search");
    state.toggle_turn();

    result
}

/// Updates best score and move if new score is better.
/// Returns true if best_score was updated.
fn update_best<M: Clone>(
    score: i16,
    candidate_move: &M,
    maximizing_player: bool,
    best_score: &mut i16,
    best_move: &mut Option<M>,
) -> bool {
    let is_better = if maximizing_player {
        score > *best_score
    } else {
        score < *best_score
    };

    if is_better {
        *best_score = score;
        *best_move = Some(candidate_move.clone());
    }
    is_better
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
    G::MoveList: Sync,
    E: Evaluator<S>,
    O: MoveOrderer<S, G::Move>,
{
    debug!("alpha-beta search depth: {}", context.search_depth());
    let target_depth = context.search_depth();

    if target_depth < 1 {
        return Err(SearchError::DepthTooLow);
    }

    let start = Instant::now();
    let current_player_is_maximizing = state.is_maximizing_player();
    let mut candidates = move_generator.generate_moves(state);

    if candidates.is_empty() {
        return Err(SearchError::NoAvailableMoves);
    }

    move_orderer.order_moves(candidates.as_mut(), state);

    let hash = state.position_hash();

    // Iterative deepening: search at increasing depths, using previous results for move ordering
    let mut best_move = None;
    let mut best_score = if current_player_is_maximizing {
        i16::MIN
    } else {
        i16::MAX
    };

    for depth in 1..=target_depth {
        // Check if we already have an exact result at this depth from TT
        if let Some((score, Some(ref mv))) =
            context
                .transposition_table
                .probe(hash, depth, i16::MIN, i16::MAX)
        {
            if candidates.as_ref().iter().any(|c| c == mv) {
                debug!("Using transposition table hit at depth {}", depth);
                best_move = Some(mv.clone());
                best_score = score;
                // Continue to next depth to ensure we search to target_depth
                continue;
            }
        }

        // Reorder moves: prioritize best move from previous iteration
        if let Some(ref prev_best) = best_move {
            if let Some(pos) = candidates.as_mut().iter().position(|m| m == prev_best) {
                if pos > 0 {
                    candidates.as_mut()[0..=pos].rotate_right(1);
                }
            }
        }

        let (score, move_found) = if context.is_parallel() {
            search_root_parallel(
                context,
                state,
                move_generator,
                evaluator,
                move_orderer,
                &candidates,
                depth,
                current_player_is_maximizing,
            )?
        } else {
            search_root_sequential(
                context,
                state,
                move_generator,
                evaluator,
                move_orderer,
                &candidates,
                depth,
                current_player_is_maximizing,
            )?
        };

        if let Some(mv) = move_found {
            best_move = Some(mv);
            best_score = score;
        }
    }

    let best_move = best_move.ok_or(SearchError::NoAvailableMoves)?;

    context.transposition_table.store(
        hash,
        best_score,
        target_depth,
        BoundType::Exact,
        Some(best_move.clone()),
    );

    context.stats.record_result(best_score, start.elapsed());

    Ok(best_move)
}

#[allow(clippy::too_many_arguments)]
fn search_root_sequential<S, G, E, O, C>(
    context: &SearchContext<G::Move>,
    state: &mut S,
    move_generator: &G,
    evaluator: &E,
    move_orderer: &O,
    candidates: &C,
    depth: u8,
    maximizing_player: bool,
) -> Result<(i16, Option<G::Move>), SearchError>
where
    S: GameState,
    G: MoveGenerator<S, MoveList = C>,
    G::Move: GameMove<State = S>,
    C: MoveCollection<G::Move>,
    E: Evaluator<S>,
    O: MoveOrderer<S, G::Move>,
{
    let mut best_score = if maximizing_player {
        i16::MIN
    } else {
        i16::MAX
    };
    let mut best_move = None;

    for game_move in candidates.as_ref().iter() {
        let score = with_move_applied(game_move, state, |state| {
            alpha_beta_minimax(
                context,
                state,
                move_generator,
                evaluator,
                move_orderer,
                depth - 1,
                0, // ply starts at 0 for root
                i16::MIN,
                i16::MAX,
                !maximizing_player,
            )
        })?;

        update_best(
            score,
            game_move,
            maximizing_player,
            &mut best_score,
            &mut best_move,
        );
    }

    Ok((best_score, best_move))
}

#[allow(clippy::too_many_arguments)]
fn search_root_parallel<S, G, E, O, C>(
    context: &SearchContext<G::Move>,
    state: &S,
    move_generator: &G,
    evaluator: &E,
    move_orderer: &O,
    candidates: &C,
    depth: u8,
    maximizing_player: bool,
) -> Result<(i16, Option<G::Move>), SearchError>
where
    S: GameState + Clone,
    G: MoveGenerator<S, MoveList = C> + Sync,
    G::Move: GameMove<State = S>,
    C: MoveCollection<G::Move> + Sync,
    E: Evaluator<S> + Sync,
    O: MoveOrderer<S, G::Move> + Sync,
{
    let results: Vec<_> = candidates
        .as_ref()
        .par_iter()
        .map(|game_move| {
            let mut cloned_state = state.clone();

            let score = with_move_applied(game_move, &mut cloned_state, |state| {
                alpha_beta_minimax(
                    context,
                    state,
                    move_generator,
                    evaluator,
                    move_orderer,
                    depth - 1,
                    0, // ply starts at 0 for root
                    i16::MIN,
                    i16::MAX,
                    !maximizing_player,
                )
            })
            .expect("minimax should succeed in parallel search");

            (score, game_move.clone())
        })
        .collect();

    let mut best_score = if maximizing_player {
        i16::MIN
    } else {
        i16::MAX
    };
    let mut best_move = None;

    for (score, game_move) in results {
        update_best(
            score,
            &game_move,
            maximizing_player,
            &mut best_score,
            &mut best_move,
        );
    }

    Ok((best_score, best_move))
}

const MAX_QUIESCENCE_DEPTH: u8 = 8;

#[allow(clippy::too_many_arguments, clippy::only_used_in_recursion)]
fn quiescence_search<S, G, E, O>(
    context: &SearchContext<G::Move>,
    state: &mut S,
    move_generator: &G,
    evaluator: &E,
    move_orderer: &O,
    mut alpha: i16,
    beta: i16,
    maximizing_player: bool,
    qdepth: u8,
) -> Result<i16, SearchError>
where
    S: GameState,
    G: MoveGenerator<S>,
    G::Move: GameMove<State = S>,
    E: Evaluator<S>,
    O: MoveOrderer<S, G::Move>,
{
    context.increment_position_count();

    if qdepth >= MAX_QUIESCENCE_DEPTH {
        return Ok(evaluator.evaluate(state, 0));
    }

    let stand_pat = evaluator.evaluate(state, 0);
    if stand_pat >= beta {
        return Ok(beta);
    }
    if stand_pat > alpha {
        alpha = stand_pat;
    }

    let candidates = move_generator.generate_moves(state);
    if candidates.is_empty() {
        return Ok(stand_pat);
    }

    let mut tactical_moves: Vec<G::Move> = candidates
        .as_ref()
        .iter()
        .filter(|mv| mv.is_tactical(state))
        .cloned()
        .collect();

    if tactical_moves.is_empty() {
        return Ok(stand_pat);
    }

    move_orderer.order_moves(&mut tactical_moves, state);

    let mut best_score = stand_pat;

    for game_move in tactical_moves.iter() {
        game_move
            .apply(state)
            .expect("move application should succeed in quiescence");
        state.toggle_turn();

        let score = -quiescence_search(
            context,
            state,
            move_generator,
            evaluator,
            move_orderer,
            -beta,
            -alpha,
            !maximizing_player,
            qdepth + 1,
        )?;

        game_move
            .undo(state)
            .expect("move undo should succeed in quiescence");
        state.toggle_turn();

        if score >= beta {
            return Ok(beta);
        }
        if score > alpha {
            alpha = score;
        }
        if score > best_score {
            best_score = score;
        }
    }

    Ok(best_score)
}

#[allow(clippy::too_many_arguments)]
fn alpha_beta_minimax<S, G, E, O>(
    context: &SearchContext<G::Move>,
    state: &mut S,
    move_generator: &G,
    evaluator: &E,
    move_orderer: &O,
    depth: u8,
    ply: u8,
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

    // Probe TT once for both cutoff score and PV move
    let (cutoff_score, tt_move) = context
        .transposition_table
        .probe_with_move(hash, depth, alpha, beta);

    // Early return if TT allows cutoff
    if let Some(score) = cutoff_score {
        return Ok(score);
    }

    if depth == 0 {
        return quiescence_search(
            context,
            state,
            move_generator,
            evaluator,
            move_orderer,
            alpha,
            beta,
            maximizing_player,
            0,
        );
    }

    // Null move pruning: give opponent a free move; if they still can't beat beta, prune branch
    if depth >= 3 && !state.is_in_check() && !state.is_endgame() {
        const NULL_MOVE_REDUCTION: u8 = 2;
        state.toggle_turn();
        let null_score = -alpha_beta_minimax(
            context,
            state,
            move_generator,
            evaluator,
            move_orderer,
            depth - 1 - NULL_MOVE_REDUCTION,
            ply + 1,
            -beta,
            -beta + 1,
            !maximizing_player,
        )?;
        state.toggle_turn();

        if null_score >= beta {
            return Ok(beta);
        }
    }

    let mut candidates = move_generator.generate_moves(state);

    if candidates.is_empty() {
        let score = evaluator.evaluate(state, depth);
        return Ok(score);
    }

    move_orderer.order_moves(candidates.as_mut(), state);

    let moves_slice = candidates.as_mut();

    // Boost PV move from TT to front first (highest priority)
    if let Some(ref pv_move) = tt_move {
        if let Some(pos) = moves_slice.iter().position(|m| m == pv_move) {
            // Only reorder if not already first
            if pos > 0 {
                moves_slice[0..=pos].rotate_right(1);
            }
        }
    }

    // Boost killer moves to front after PV move
    // Start from position 1 if PV move exists, 0 otherwise
    let killer_start = if tt_move.is_some() { 1 } else { 0 };
    let killers = context.get_killers(ply);

    for killer in killers.iter().flatten().rev() {
        if let Some(pos) = moves_slice.iter().position(|m| m == killer) {
            // Only move if it's beyond the killer insertion point and not the PV move
            if pos > killer_start && Some(killer) != tt_move.as_ref() {
                moves_slice[killer_start..=pos].rotate_right(1);
            }
        }
    }

    let mut best_move = None;
    let mut best_score = if maximizing_player {
        i16::MIN
    } else {
        i16::MAX
    };
    let original_alpha = alpha;

    for game_move in candidates.as_ref().iter() {
        let score = with_move_applied(game_move, state, |state| {
            alpha_beta_minimax(
                context,
                state,
                move_generator,
                evaluator,
                move_orderer,
                depth - 1,
                ply + 1,
                alpha,
                beta,
                !maximizing_player,
            )
        })?;

        update_best(
            score,
            game_move,
            maximizing_player,
            &mut best_score,
            &mut best_move,
        );

        if maximizing_player {
            alpha = max(alpha, score);
        } else {
            beta = min(beta, score);
        }

        if beta <= alpha {
            // Beta cutoff - store killer move and notify move orderer
            context.store_killer(ply, game_move.clone());
            move_orderer.record_cutoff(game_move, state, depth);
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
