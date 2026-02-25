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
//! ## Reverse Futility Pruning (RFP)
//! At shallow depths (controlled by `Evaluator::rfp_margin`), if the static evaluation
//! already exceeds the opponent's bound by a depth-dependent margin, the entire subtree is
//! pruned. This avoids generating and searching moves in positions where the side to move
//! is so far ahead that no legal response could change the outcome within the remaining
//! search depth. Disabled when in check or in endgame positions (same safety conditions
//! as null move pruning).
//!
//! ## Futility Pruning
//! The counterpart of RFP that operates per-move rather than per-node. At shallow depths,
//! individual quiet (non-tactical) moves are skipped if the static eval plus the depth
//! margin still cannot reach alpha (for the maximizer) or beta (for the minimizer). The
//! first move is never pruned, and tactical moves (captures, promotions) are always
//! searched. Uses the same margin values as RFP via `Evaluator::rfp_margin`.
//!
//! ## Late Move Reductions (LMR)
//! After the first few moves at each node, later quiet moves are searched at reduced depth
//! under the assumption that well-ordered moves tend to be best. Uses a logarithmic formula
//! `reduction = 1 + ln(depth) * ln(move_count) / 2` that scales reductions with both depth
//! and move index. If the reduced search returns a score that improves on the current bound,
//! a full-depth re-search is performed to verify the result.
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
#[cfg(feature = "instrumentation")]
use tracing::instrument;

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
    null_move_attempts: AtomicUsize,
    null_move_cutoffs: AtomicUsize,
    rfp_attempts: AtomicUsize,
    rfp_cutoffs: AtomicUsize,
    fp_attempts: AtomicUsize,
    fp_cutoffs: AtomicUsize,
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
            null_move_attempts: AtomicUsize::new(0),
            null_move_cutoffs: AtomicUsize::new(0),
            rfp_attempts: AtomicUsize::new(0),
            rfp_cutoffs: AtomicUsize::new(0),
            fp_attempts: AtomicUsize::new(0),
            fp_cutoffs: AtomicUsize::new(0),
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

    fn increment_null_move_attempts(&self) {
        self.null_move_attempts.fetch_add(1, Ordering::SeqCst);
    }

    fn increment_null_move_cutoffs(&self) {
        self.null_move_cutoffs.fetch_add(1, Ordering::SeqCst);
    }

    fn increment_rfp_attempts(&self) {
        self.rfp_attempts.fetch_add(1, Ordering::SeqCst);
    }

    fn increment_rfp_cutoffs(&self) {
        self.rfp_cutoffs.fetch_add(1, Ordering::SeqCst);
    }

    fn increment_fp_attempts(&self) {
        self.fp_attempts.fetch_add(1, Ordering::SeqCst);
    }

    fn increment_fp_cutoffs(&self) {
        self.fp_cutoffs.fetch_add(1, Ordering::SeqCst);
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
        self.null_move_attempts.store(0, Ordering::SeqCst);
        self.null_move_cutoffs.store(0, Ordering::SeqCst);
        self.rfp_attempts.store(0, Ordering::SeqCst);
        self.rfp_cutoffs.store(0, Ordering::SeqCst);
        self.fp_attempts.store(0, Ordering::SeqCst);
        self.fp_cutoffs.store(0, Ordering::SeqCst);
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

    fn null_move_attempts(&self) -> usize {
        self.null_move_attempts.load(Ordering::SeqCst)
    }

    fn null_move_cutoffs(&self) -> usize {
        self.null_move_cutoffs.load(Ordering::SeqCst)
    }

    fn rfp_attempts(&self) -> usize {
        self.rfp_attempts.load(Ordering::SeqCst)
    }

    fn rfp_cutoffs(&self) -> usize {
        self.rfp_cutoffs.load(Ordering::SeqCst)
    }

    fn fp_attempts(&self) -> usize {
        self.fp_attempts.load(Ordering::SeqCst)
    }

    fn fp_cutoffs(&self) -> usize {
        self.fp_cutoffs.load(Ordering::SeqCst)
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

    /// Reset stats and killers but keep transposition table entries.
    /// Useful for benchmarking multiple positions with shared TT.
    pub fn reset_stats_keep_tt(&mut self) {
        self.stats.reset();
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

    pub fn tt_depth_rejected(&self) -> usize {
        self.transposition_table.depth_rejected()
    }

    pub fn tt_bound_rejected(&self) -> usize {
        self.transposition_table.bound_rejected()
    }

    pub fn tt_overwrites(&self) -> usize {
        self.transposition_table.overwrites()
    }

    pub fn tt_size(&self) -> usize {
        self.transposition_table.size()
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

    pub fn null_move_attempts(&self) -> usize {
        self.stats.null_move_attempts()
    }

    pub fn null_move_cutoffs(&self) -> usize {
        self.stats.null_move_cutoffs()
    }

    pub fn rfp_attempts(&self) -> usize {
        self.stats.rfp_attempts()
    }

    pub fn rfp_cutoffs(&self) -> usize {
        self.stats.rfp_cutoffs()
    }

    pub fn fp_attempts(&self) -> usize {
        self.stats.fp_attempts()
    }

    pub fn fp_cutoffs(&self) -> usize {
        self.stats.fp_cutoffs()
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

    fn increment_null_move_attempts(&self) {
        self.stats.increment_null_move_attempts();
    }

    fn increment_null_move_cutoffs(&self) {
        self.stats.increment_null_move_cutoffs();
    }

    fn increment_rfp_attempts(&self) {
        self.stats.increment_rfp_attempts();
    }

    fn increment_rfp_cutoffs(&self) {
        self.stats.increment_rfp_cutoffs();
    }

    fn increment_fp_attempts(&self) {
        self.stats.increment_fp_attempts();
    }

    fn increment_fp_cutoffs(&self) {
        self.stats.increment_fp_cutoffs();
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
#[cfg_attr(feature = "instrumentation", instrument(skip_all))]
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

    const ASPIRATION_WINDOW: i16 = 50;

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

        // Aspiration window: use narrow window around previous score at depth 4+
        let (mut window_alpha, mut window_beta) =
            if depth >= 4 && best_score > i16::MIN / 2 + 100 && best_score < i16::MAX / 2 - 100 {
                (
                    best_score.saturating_sub(ASPIRATION_WINDOW),
                    best_score.saturating_add(ASPIRATION_WINDOW),
                )
            } else {
                (i16::MIN, i16::MAX)
            };

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
                window_alpha,
                window_beta,
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
                window_alpha,
                window_beta,
            )?
        };

        // If score falls outside aspiration window, re-search with full window
        let (score, move_found) = if score <= window_alpha || score >= window_beta {
            window_alpha = i16::MIN;
            window_beta = i16::MAX;
            if context.is_parallel() {
                search_root_parallel(
                    context,
                    state,
                    move_generator,
                    evaluator,
                    move_orderer,
                    &candidates,
                    depth,
                    current_player_is_maximizing,
                    window_alpha,
                    window_beta,
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
                    window_alpha,
                    window_beta,
                )?
            }
        } else {
            (score, move_found)
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
    alpha: i16,
    beta: i16,
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
    let mut current_alpha = alpha;
    let mut current_beta = beta;

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
                current_alpha,
                current_beta,
                !maximizing_player,
                true,
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
            current_alpha = max(current_alpha, score);
        } else {
            current_beta = min(current_beta, score);
        }
        if current_beta <= current_alpha {
            break;
        }
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
    alpha: i16,
    beta: i16,
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
                    alpha,
                    beta,
                    !maximizing_player,
                    true,
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
#[cfg_attr(feature = "instrumentation", instrument(skip_all))]
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
#[cfg_attr(feature = "instrumentation", instrument(skip_all))]
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
    allow_null_move: bool,
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

    // Cache the safety check for both NMP and RFP to avoid redundant check detection.
    let skip_speculative_pruning = evaluator.should_skip_null_move(state);

    // Null Move Pruning: if passing still causes a beta cutoff, prune the subtree
    const NULL_MOVE_REDUCTION: u8 = 2;
    if allow_null_move && depth >= 3 && !skip_speculative_pruning {
        context.increment_null_move_attempts();
        state.apply_null_move();
        let null_score = alpha_beta_minimax(
            context,
            state,
            move_generator,
            evaluator,
            move_orderer,
            depth - 1 - NULL_MOVE_REDUCTION,
            ply + 1,
            alpha,
            beta,
            !maximizing_player,
            false,
        )?;
        state.undo_null_move();

        if maximizing_player && null_score >= beta {
            context.increment_null_move_cutoffs();
            return Ok(beta);
        }
        if !maximizing_player && null_score <= alpha {
            context.increment_null_move_cutoffs();
            return Ok(alpha);
        }
    }

    // Compute static eval lazily for RFP and futility pruning at shallow depths.
    // Cached here to avoid redundant evaluation calls.
    let rfp_margin = evaluator.rfp_margin(depth);
    let static_eval = if depth > 0 && !skip_speculative_pruning && rfp_margin.is_some() {
        Some(evaluator.evaluate(state, depth))
    } else {
        None
    };

    // Reverse Futility Pruning: if static eval is far above beta (or below alpha),
    // the position is so good that no move will fail to maintain the advantage.
    // Returns the bound (beta/alpha) consistent with NMP convention.
    if let (Some(margin), Some(eval)) = (rfp_margin, static_eval) {
        context.increment_rfp_attempts();
        if maximizing_player && eval.saturating_sub(margin) >= beta {
            context.increment_rfp_cutoffs();
            return Ok(beta);
        }
        if !maximizing_player && eval.saturating_add(margin) <= alpha {
            context.increment_rfp_cutoffs();
            return Ok(alpha);
        }
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
    let mut move_count = 0;

    // Futility pruning: at shallow depths, skip quiet moves that can't reach the bound.
    // Reuses the same margin and static_eval already computed for RFP above — both use
    // rfp_margin(depth) at the same depth, so whenever do_futility is true, static_eval
    // is guaranteed to already be Some from the RFP computation.
    let futility_margin = evaluator.rfp_margin(depth);
    let do_futility = futility_margin.is_some() && !skip_speculative_pruning;

    for game_move in candidates.as_ref().iter() {
        move_count += 1;
        let is_first_move = move_count == 1;

        let score = if is_first_move {
            // Search first move with full window
            with_move_applied(game_move, state, |state| {
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
                    true,
                )
            })?
        } else {
            // Late Move Reductions (LMR): Reduce depth for late non-tactical moves
            let is_tactical = game_move.is_tactical(state);

            // Futility pruning: skip quiet moves at shallow depths that can't
            // possibly raise the score to alpha (maximizing) or lower it to
            // beta (minimizing).
            if do_futility && !is_tactical {
                if let (Some(eval), Some(margin)) = (static_eval, futility_margin) {
                    context.increment_fp_attempts();
                    let dominated = if maximizing_player {
                        eval.saturating_add(margin) <= alpha
                    } else {
                        eval.saturating_sub(margin) >= beta
                    };
                    if dominated {
                        context.increment_fp_cutoffs();
                        continue;
                    }
                }
            }
            let do_lmr = depth >= 3 && move_count > 3 && !is_tactical;
            let reduction = if do_lmr {
                // Logarithmic reduction: deeper depths and later moves get larger reductions.
                let r = 1.0 + (depth as f64).ln() * (move_count as f64).ln() / 2.0;
                // Clamp: at least 1, at most depth-2 (always search at least 1 ply).
                (r.floor() as u8).clamp(1, depth.saturating_sub(2).max(1))
            } else {
                0
            };

            // Try reduced depth search first (if LMR applies)
            let reduced_score = if do_lmr {
                Some(with_move_applied(game_move, state, |state| {
                    alpha_beta_minimax(
                        context,
                        state,
                        move_generator,
                        evaluator,
                        move_orderer,
                        depth - 1 - reduction,
                        ply + 1,
                        if maximizing_player { alpha } else { beta - 1 },
                        if maximizing_player { alpha + 1 } else { beta },
                        !maximizing_player,
                        true,
                    )
                })?)
            } else {
                None
            };

            // PV-Search: Try null window search (skip if LMR confirmed move is uninteresting).
            // For maximizing nodes, "uninteresting" means rs <= alpha (failed low).
            // For minimizing nodes, "uninteresting" means rs >= beta (failed high).
            let null_window_score = if let Some(rs) = reduced_score {
                let move_is_uninteresting = if maximizing_player {
                    rs <= alpha
                } else {
                    rs >= beta
                };
                if move_is_uninteresting {
                    // Reduced search confirmed this move won't improve the bound
                    rs
                } else {
                    // Reduced search suggests the move may be interesting, verify at full depth
                    with_move_applied(game_move, state, |state| {
                        alpha_beta_minimax(
                            context,
                            state,
                            move_generator,
                            evaluator,
                            move_orderer,
                            depth - 1,
                            ply + 1,
                            if maximizing_player { alpha } else { beta - 1 },
                            if maximizing_player { alpha + 1 } else { beta },
                            !maximizing_player,
                            true,
                        )
                    })?
                }
            } else {
                // No LMR, do regular null window search
                with_move_applied(game_move, state, |state| {
                    alpha_beta_minimax(
                        context,
                        state,
                        move_generator,
                        evaluator,
                        move_orderer,
                        depth - 1,
                        ply + 1,
                        if maximizing_player { alpha } else { beta - 1 },
                        if maximizing_player { alpha + 1 } else { beta },
                        !maximizing_player,
                        true,
                    )
                })?
            };

            // If null window search fails (score is in (alpha, beta)), re-search with full window
            if null_window_score > alpha && null_window_score < beta {
                with_move_applied(game_move, state, |state| {
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
                        true,
                    )
                })?
            } else {
                null_window_score
            }
        };

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
