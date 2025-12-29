//! Alpha-beta search algorithm implementation.

use std::cmp::{max, min};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use log::debug;
use rayon::prelude::*;
use thiserror::Error;

use super::transposition_table::{BoundType, TranspositionTable};
use super::{Evaluator, GameMove, GameState, MoveCollection, MoveGenerator, MoveOrderer};

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
    parallel: bool,
    killer_moves: Mutex<Vec<[Option<M>; 2]>>, // 2 killer moves per ply
}

impl<M: Clone + Send + Sync> SearchContext<M> {
    pub fn new(depth: u8) -> Self {
        Self {
            search_depth: depth,
            searched_position_count: AtomicUsize::new(0),
            last_score: None,
            last_search_duration: None,
            transposition_table: TranspositionTable::default(),
            parallel: false,
            killer_moves: Mutex::new(vec![[None, None]; depth as usize + 1]),
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
            killer_moves: Mutex::new(vec![[None, None]; depth as usize + 1]),
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

    pub fn store_killer(&self, ply: u8, killer: M) {
        let ply = ply as usize;
        if let Ok(mut killers) = self.killer_moves.lock() {
            if ply < killers.len() {
                // Shift killers: new killer becomes first, first becomes second
                let old_first = killers[ply][0].clone();
                killers[ply][1] = old_first;
                killers[ply][0] = Some(killer);
            }
        }
    }

    pub fn get_killers(&self, ply: u8) -> [Option<M>; 2] {
        let ply = ply as usize;
        if let Ok(killers) = self.killer_moves.lock() {
            if ply < killers.len() {
                return killers[ply].clone();
            }
        }
        [None, None]
    }

    pub fn clear_killers(&mut self) {
        if let Ok(mut killers) = self.killer_moves.lock() {
            for killer in killers.iter_mut() {
                *killer = [None, None];
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
    let depth = context.search_depth();

    if depth < 1 {
        return Err(SearchError::DepthTooLow);
    }

    let start = Instant::now();
    let current_player_is_maximizing = state.is_maximizing_player();
    let mut candidates = move_generator.generate_moves(state);

    move_orderer.order_moves(candidates.as_mut(), state);

    let hash = state.position_hash();
    if let Some((score, Some(ref mv))) =
        context
            .transposition_table
            .probe(hash, depth, i16::MIN, i16::MAX)
    {
        if candidates.as_ref().iter().any(|c| c == mv) {
            debug!("Using transposition table hit");
            context.last_score = Some(score);
            context.last_search_duration = Some(start.elapsed());
            return Ok(mv.clone());
        }
    }

    let (best_score, best_move) = if context.parallel {
        // Parallel search: clone state for each thread
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
        // Sequential search: use move apply/undo (no cloning overhead)
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

    let best_move = best_move.ok_or(SearchError::NoAvailableMoves)?;

    context.transposition_table.store(
        hash,
        best_score,
        depth,
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

    // Boost killer moves to front after initial ordering
    let killers = context.get_killers(ply);
    let moves_slice = candidates.as_mut();
    for killer in killers.iter().flatten().rev() {
        if let Some(pos) = moves_slice.iter().position(|m| m == killer) {
            if pos > 0 {
                moves_slice[0..=pos].rotate_right(1);
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
