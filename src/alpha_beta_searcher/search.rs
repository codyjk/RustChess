//! Alpha-beta search algorithm implementation.
//!
//! # Core Algorithm
//!
//! Alpha-beta pruning is an optimization of minimax search that maintains a window [alpha, beta]
//! representing the range of scores that matter. Moves that fall outside this window can be
//! pruned without affecting the final result. The algorithm guarantees finding the same move as
//! minimax but explores fewer nodes.
//!
//! # Optimizations
//!
//! ## Iterative Deepening
//! Searches at increasing depths (1..target_depth), using results from shallower searches to
//! improve move ordering at deeper levels. The best move from depth N-1 (stored in the
//! transposition table) is prioritized at depth N, dramatically improving pruning efficiency.
//!
//! ## Transposition Tables
//! Caches position evaluations by Zobrist hash to avoid re-searching identical positions that
//! arise through move transpositions. Stores the score, depth, bound type (exact/upper/lower),
//! and best move for each position.
//!
//! ## Move Ordering
//! Orders moves to maximize alpha-beta cutoffs:
//! 1. PV (Principal Variation) move from transposition table
//! 2. Killer moves (moves that caused cutoffs at the same ply)
//! 3. Other heuristics (via MoveOrderer trait)
//!
//! Better move ordering leads to more cutoffs and faster search.
//!
//! ## Quiescence Search
//! Extends search beyond the nominal depth for tactical moves to avoid the horizon effect
//! where evaluation stops just before a critical sequence. Games opt in by implementing
//! `is_tactical` on their move type to identify which moves should be searched in quiescence.
//!
//! ## Parallel Search
//! Root moves can be searched in parallel using thread-local storage for killer moves to
//! eliminate lock contention.

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
    quiescence_nodes: AtomicUsize,
    tt_probes: AtomicUsize,
    tt_stores: AtomicUsize,
    tt_probe_misses: AtomicUsize,
    move_gen_calls: AtomicUsize,
    last_score: Option<i16>,
    last_duration: Option<Duration>,
}

impl SearchStats {
    fn new() -> Self {
        Self {
            position_count: AtomicUsize::new(0),
            quiescence_nodes: AtomicUsize::new(0),
            tt_probes: AtomicUsize::new(0),
            tt_stores: AtomicUsize::new(0),
            tt_probe_misses: AtomicUsize::new(0),
            move_gen_calls: AtomicUsize::new(0),
            last_score: None,
            last_duration: None,
        }
    }

    fn increment(&self) {
        self.position_count.fetch_add(1, Ordering::SeqCst);
    }

    fn increment_quiescence(&self) {
        self.quiescence_nodes.fetch_add(1, Ordering::SeqCst);
    }

    fn increment_tt_probes(&self) {
        self.tt_probes.fetch_add(1, Ordering::SeqCst);
    }

    fn increment_tt_stores(&self) {
        self.tt_stores.fetch_add(1, Ordering::SeqCst);
    }

    fn increment_tt_misses(&self) {
        self.tt_probe_misses.fetch_add(1, Ordering::SeqCst);
    }

    fn increment_move_gen(&self) {
        self.move_gen_calls.fetch_add(1, Ordering::SeqCst);
    }

    fn reset(&mut self) {
        self.last_score = None;
        self.last_duration = None;
        self.position_count.store(0, Ordering::SeqCst);
        self.quiescence_nodes.store(0, Ordering::SeqCst);
        self.tt_probes.store(0, Ordering::SeqCst);
        self.tt_stores.store(0, Ordering::SeqCst);
        self.tt_probe_misses.store(0, Ordering::SeqCst);
        self.move_gen_calls.store(0, Ordering::SeqCst);
    }

    fn record_result(&mut self, score: i16, duration: Duration) {
        self.last_score = Some(score);
        self.last_duration = Some(duration);
    }

    fn count(&self) -> usize {
        self.position_count.load(Ordering::SeqCst)
    }

    fn quiescence_nodes(&self) -> usize {
        self.quiescence_nodes.load(Ordering::SeqCst)
    }

    fn tt_probes(&self) -> usize {
        self.tt_probes.load(Ordering::SeqCst)
    }

    fn tt_stores(&self) -> usize {
        self.tt_stores.load(Ordering::SeqCst)
    }

    fn tt_probe_misses(&self) -> usize {
        self.tt_probe_misses.load(Ordering::SeqCst)
    }

    fn move_gen_calls(&self) -> usize {
        self.move_gen_calls.load(Ordering::SeqCst)
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

    pub fn tt_probes(&self) -> usize {
        self.stats.tt_probes()
    }

    pub fn move_gen_calls(&self) -> usize {
        self.stats.move_gen_calls()
    }

    pub fn quiescence_nodes(&self) -> usize {
        self.stats.quiescence_nodes()
    }

    pub fn tt_stores(&self) -> usize {
        self.stats.tt_stores()
    }

    pub fn tt_probe_misses(&self) -> usize {
        self.stats.tt_probe_misses()
    }

    fn increment_position_count(&self) {
        self.stats.increment();
    }

    fn increment_quiescence(&self) {
        self.stats.increment_quiescence();
    }

    fn increment_tt_probes(&self) {
        self.stats.increment_tt_probes();
    }

    fn increment_tt_stores(&self) {
        self.stats.increment_tt_stores();
    }

    fn increment_tt_misses(&self) {
        self.stats.increment_tt_misses();
    }

    fn increment_move_gen(&self) {
        self.stats.increment_move_gen();
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

/// Reorders moves for better alpha-beta pruning.
///
/// Priority: 1) PV move from transposition table, 2) Killer moves, 3) Other moves.
/// The PV move is placed first if present, followed by killer moves, then remaining moves.
fn reorder_moves_with_heuristics<M>(moves: &mut [M], pv_move: Option<&M>, killers: [Option<M>; 2])
where
    M: PartialEq + Clone,
{
    // Move PV to front (highest priority)
    if let Some(pv) = pv_move {
        if let Some(pos) = moves.iter().position(|m| m == pv) {
            if pos > 0 {
                moves[0..=pos].rotate_right(1);
            }
        }
    }

    // Move killers to front after PV
    let killer_start = if pv_move.is_some() { 1 } else { 0 };
    for killer in killers.iter().flatten().rev() {
        if let Some(pos) = moves.iter().position(|m| m == killer) {
            // Only move if beyond insertion point and not the PV move
            if pos > killer_start && Some(killer) != pv_move {
                moves[killer_start..=pos].rotate_right(1);
            }
        }
    }
}

/// Searches for the best move using alpha-beta pruning with iterative deepening.
///
/// This is the main entry point for the search algorithm. It performs iterative deepening,
/// searching at depths 1 through the target depth. Each iteration uses the best move from
/// the previous depth (stored in the transposition table) to improve move ordering.
///
/// # Returns
///
/// - `Ok(best_move)` - The best move found at the target depth
/// - `Err(SearchError::DepthTooLow)` - If search depth is < 1
/// - `Err(SearchError::NoAvailableMoves)` - If no legal moves available
///
/// # Examples
///
/// ```ignore
/// let mut context = SearchContext::new(6);
/// let best_move = alpha_beta_search(
///     &mut context,
///     &mut board,
///     &move_gen,
///     &evaluator,
///     &move_orderer,
/// )?;
/// ```
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

    context.increment_tt_stores();
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

/// Quiescence search to avoid the horizon effect.
///
/// Extends the search beyond the nominal depth by only considering tactical moves.
/// This prevents the evaluation from being distorted by stopping the search in the
/// middle of a tactical sequence.
///
/// The search continues until reaching a "quiet" position where no tactical moves
/// are available, or until MAX_QUIESCENCE_DEPTH is reached.
///
/// # Parameters
///
/// - `alpha` - Lower bound of search window
/// - `beta` - Upper bound of search window
/// - `maximizing_player` - True if current player wants to maximize score
/// - `qdepth` - Current quiescence depth (limited to MAX_QUIESCENCE_DEPTH)
///
/// # Returns
///
/// The evaluation score for this position within the [alpha, beta] window.
#[allow(clippy::too_many_arguments, clippy::only_used_in_recursion)]
fn quiescence_search<S, G, E, O>(
    context: &SearchContext<G::Move>,
    state: &mut S,
    hash: u64,
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
    context.increment_quiescence();

    // Probe TT for cached quiescence result
    context.increment_tt_probes();
    let (cutoff_score, _tt_move) = context
        .transposition_table
        .probe_with_move(hash, qdepth, alpha, beta);

    // Track miss
    if cutoff_score.is_none() {
        context.increment_tt_misses();
    }

    // Early return on TT hit
    if let Some(score) = cutoff_score {
        return Ok(score);
    }

    // Save original alpha for bound type determination
    let original_alpha = alpha;

    if qdepth >= MAX_QUIESCENCE_DEPTH {
        let score = evaluator.evaluate(state, 0);
        context.increment_tt_stores();
        context
            .transposition_table
            .store(hash, score, qdepth, BoundType::Exact, None);
        return Ok(score);
    }

    let stand_pat = evaluator.evaluate(state, 0);
    if stand_pat >= beta {
        context.increment_tt_stores();
        context
            .transposition_table
            .store(hash, beta, qdepth, BoundType::Lower, None);
        return Ok(beta);
    }
    if stand_pat > alpha {
        alpha = stand_pat;
    }

    context.increment_move_gen();
    let candidates = move_generator.generate_moves(state);
    if candidates.is_empty() {
        context.increment_tt_stores();
        context
            .transposition_table
            .store(hash, stand_pat, qdepth, BoundType::Exact, None);
        return Ok(stand_pat);
    }

    let mut tactical_moves: Vec<G::Move> = candidates
        .as_ref()
        .iter()
        .filter(|mv| mv.is_tactical(state))
        .cloned()
        .collect();

    if tactical_moves.is_empty() {
        context.increment_tt_stores();
        context
            .transposition_table
            .store(hash, stand_pat, qdepth, BoundType::Exact, None);
        return Ok(stand_pat);
    }

    move_orderer.order_moves(&mut tactical_moves, state);

    let mut best_score = stand_pat;

    // Delta pruning: get maximum possible tactical gain
    let max_gain = evaluator.max_tactical_gain(state);

    for game_move in tactical_moves.iter() {
        // Delta pruning: skip moves that cannot possibly raise alpha
        // Only apply if max_gain is reasonable (not i16::MAX which means no pruning)
        if max_gain < i16::MAX {
            // Even if we gain the maximum possible (e.g., capture queen), we still can't reach alpha
            if let Some(optimistic_score) = stand_pat.checked_add(max_gain) {
                if optimistic_score < alpha {
                    // All remaining moves are futile
                    break;
                }
            }
        }

        game_move
            .apply(state)
            .expect("move application should succeed in quiescence");
        state.toggle_turn();

        let child_hash = state.position_hash();
        let score = -quiescence_search(
            context,
            state,
            child_hash,
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
            context.increment_tt_stores();
            context
                .transposition_table
                .store(hash, beta, qdepth, BoundType::Lower, None);
            return Ok(beta);
        }
        if score > alpha {
            alpha = score;
        }
        if score > best_score {
            best_score = score;
        }
    }

    // Store result with appropriate bound type
    let bound_type = if best_score <= original_alpha {
        BoundType::Upper
    } else {
        BoundType::Exact
    };
    context.increment_tt_stores();
    context
        .transposition_table
        .store(hash, best_score, qdepth, bound_type, None);

    Ok(best_score)
}

/// Core alpha-beta minimax search with pruning.
///
/// Recursively searches the game tree using alpha-beta pruning. The [alpha, beta] window
/// represents the range of scores that matter - moves outside this window can be pruned.
///
/// # Search Optimizations
///
/// - **Transposition Table Lookup**: Checks for cached results at this position
/// - **Move Ordering**: Prioritizes PV move, killer moves, then other moves
/// - **Quiescence Extension**: Calls quiescence_search at depth 0 to avoid horizon effect
///
/// # Parameters
///
/// - `depth` - Remaining search depth (decrements each ply)
/// - `ply` - Current distance from root (increments each ply, used for killer moves)
/// - `alpha` - Lower bound of search window
/// - `beta` - Upper bound of search window
/// - `maximizing_player` - True if current player wants to maximize score
///
/// # Returns
///
/// The evaluation score for this position within the [alpha, beta] window.
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
    context.increment_tt_probes();
    let (cutoff_score, tt_move) = context
        .transposition_table
        .probe_with_move(hash, depth, alpha, beta);

    // Track if we got a TT miss
    if cutoff_score.is_none() && tt_move.is_none() {
        context.increment_tt_misses();
    }

    // Early return if TT allows cutoff
    if let Some(score) = cutoff_score {
        return Ok(score);
    }

    if depth == 0 {
        return quiescence_search(
            context,
            state,
            hash,
            move_generator,
            evaluator,
            move_orderer,
            alpha,
            beta,
            maximizing_player,
            0,
        );
    }

    context.increment_move_gen();
    let mut candidates = move_generator.generate_moves(state);

    if candidates.is_empty() {
        let score = evaluator.evaluate(state, depth);
        return Ok(score);
    }

    move_orderer.order_moves(candidates.as_mut(), state);

    let killers = context.get_killers(ply);
    reorder_moves_with_heuristics(candidates.as_mut(), tt_move.as_ref(), killers);

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

    context.increment_tt_stores();
    context
        .transposition_table
        .store(hash, best_score, depth, bound_type, best_move);

    Ok(best_score)
}
