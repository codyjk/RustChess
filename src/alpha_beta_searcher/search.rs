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
use thread_local::ThreadLocal;

use super::transposition_table::{BoundType, TranspositionTable};
use super::{Evaluator, GameMove, GameState, MoveCollection, MoveGenerator, MoveOrderer};

type KillerMovePair = [Option<Box<dyn std::any::Any + Send + Sync>>; 2];
type KillerMovesVec = Vec<KillerMovePair>;
type KillerMovesStorage = std::cell::RefCell<Option<KillerMovesVec>>;
static KILLER_MOVES: ThreadLocal<KillerMovesStorage> = ThreadLocal::new();

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("no available moves")]
    NoAvailableMoves,
    #[error("depth must be at least 1")]
    DepthTooLow,
}

pub struct SearchContext<M: Clone + Send + Sync + 'static> {
    search_depth: u8,
    searched_position_count: AtomicUsize,
    last_score: Option<i16>,
    last_search_duration: Option<Duration>,
    transposition_table: TranspositionTable<M>,
    parallel: bool,
}

impl<M: Clone + Send + Sync + 'static> SearchContext<M> {
    pub fn new(depth: u8) -> Self {
        Self {
            search_depth: depth,
            searched_position_count: AtomicUsize::new(0),
            last_score: None,
            last_search_duration: None,
            transposition_table: TranspositionTable::default(),
            parallel: true,
        }
    }

    pub fn with_parallel(depth: u8, parallel: bool) -> Self {
        Self {
            search_depth: depth,
            searched_position_count: AtomicUsize::new(0),
            last_score: None,
            last_search_duration: None,
            transposition_table: TranspositionTable::default(),
            parallel,
        }
    }

    pub fn set_parallel(&mut self, parallel: bool) {
        self.parallel = parallel;
    }

    pub fn is_parallel(&self) -> bool {
        self.parallel
    }

    pub fn reset_stats(&mut self) {
        self.last_score = None;
        self.last_search_duration = None;
        self.searched_position_count.store(0, Ordering::SeqCst);
        self.transposition_table.clear();
        self.clear_killers();
    }

    fn ensure_killer_storage(&self, max_ply: usize) {
        let storage = KILLER_MOVES.get_or(|| std::cell::RefCell::new(None));
        let mut storage_ref = storage.borrow_mut();
        if storage_ref.is_none() {
            let mut vec = Vec::with_capacity(max_ply + 1);
            for _ in 0..=max_ply {
                vec.push([
                    None::<Box<dyn std::any::Any + Send + Sync>>,
                    None::<Box<dyn std::any::Any + Send + Sync>>,
                ]);
            }
            *storage_ref = Some(vec);
        } else if let Some(ref killers) = *storage_ref {
            if killers.len() <= max_ply {
                let mut vec = Vec::with_capacity(max_ply + 1);
                for _ in 0..=max_ply {
                    vec.push([
                        None::<Box<dyn std::any::Any + Send + Sync>>,
                        None::<Box<dyn std::any::Any + Send + Sync>>,
                    ]);
                }
                *storage_ref = Some(vec);
            }
        }
    }

    pub fn store_killer(&self, ply: u8, killer: M) {
        let ply = ply as usize;
        let max_depth = self.search_depth as usize;
        self.ensure_killer_storage(max_depth);

        let storage = KILLER_MOVES.get().expect("storage should be initialized");
        if let Some(ref mut killers) = *storage.borrow_mut() {
            if ply < killers.len() {
                let old_first = killers[ply][0].take();
                killers[ply][1] = old_first;
                killers[ply][0] = Some(Box::new(killer));
            }
        }
    }

    pub fn get_killers(&self, ply: u8) -> [Option<M>; 2] {
        let ply = ply as usize;
        let max_depth = self.search_depth as usize;
        self.ensure_killer_storage(max_depth);

        let storage = KILLER_MOVES.get().expect("storage should be initialized");
        if let Some(ref killers) = *storage.borrow() {
            if ply < killers.len() {
                let mut result = [None, None];
                for (i, stored) in killers[ply].iter().enumerate() {
                    if let Some(boxed) = stored {
                        if let Some(killer) = boxed.downcast_ref::<M>() {
                            result[i] = Some(killer.clone());
                        }
                    }
                }
                return result;
            }
        }
        [None, None]
    }

    pub fn clear_killers(&mut self) {
        if let Some(storage) = KILLER_MOVES.get() {
            if let Some(ref mut killers) = *storage.borrow_mut() {
                for killer in killers.iter_mut() {
                    *killer = [
                        None::<Box<dyn std::any::Any + Send + Sync>>,
                        None::<Box<dyn std::any::Any + Send + Sync>>,
                    ];
                }
            }
        }
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

    pub fn tt_hits(&self) -> usize {
        self.transposition_table.hits()
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

        let (score, move_found) = if context.parallel {
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

    context.last_score = Some(best_score);
    context.last_search_duration = Some(start.elapsed());

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
            0, // ply starts at 0 for root
            i16::MIN,
            i16::MAX,
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
        } else if score < best_score {
            best_score = score;
            best_move = Some(game_move.clone());
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

            game_move
                .apply(&mut cloned_state)
                .expect("move application should succeed in search");
            cloned_state.toggle_turn();

            let score = alpha_beta_minimax(
                context,
                &mut cloned_state,
                move_generator,
                evaluator,
                move_orderer,
                depth - 1,
                0, // ply starts at 0 for root
                i16::MIN,
                i16::MAX,
                !maximizing_player,
            )
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
        if maximizing_player {
            if score > best_score {
                best_score = score;
                best_move = Some(game_move);
            }
        } else if score < best_score {
            best_score = score;
            best_move = Some(game_move);
        }
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
            ply + 1,
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
            // Beta cutoff - store killer move if it's not a capture
            // We can't easily check if it's a capture generically, so we store all cutoff moves
            // The chess-specific move orderer will handle captures first anyway
            context.store_killer(ply, game_move.clone());
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
