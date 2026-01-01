//! Generic transposition table for caching search results.

use dashmap::DashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone)]
pub struct TTEntry<M: Clone> {
    pub score: i16,
    pub depth: u8,
    pub bound_type: BoundType,
    pub best_move: Option<M>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BoundType {
    Exact,
    Lower,
    Upper,
}

const DEFAULT_TT_SIZE_MB: usize = 64;

pub struct TranspositionTable<M: Clone + Send + Sync> {
    table: DashMap<u64, TTEntry<M>>,
    hits: AtomicUsize,
}

impl<M: Clone + Send + Sync> Default for TranspositionTable<M> {
    fn default() -> Self {
        Self::new(DEFAULT_TT_SIZE_MB)
    }
}

impl<M: Clone + Send + Sync> TranspositionTable<M> {
    pub fn new(size_mb: usize) -> Self {
        let entry_size = 32;
        let num_entries = (size_mb * 1024 * 1024) / entry_size;

        Self {
            table: DashMap::with_capacity(num_entries),
            hits: AtomicUsize::new(0),
        }
    }

    pub fn store(
        &self,
        hash: u64,
        score: i16,
        depth: u8,
        bound_type: BoundType,
        best_move: Option<M>,
    ) {
        let entry = TTEntry {
            score,
            depth,
            bound_type,
            best_move,
        };

        // Simple replacement strategy: always replace
        // DashMap handles concurrent access automatically
        self.table.insert(hash, entry);

        // Optional: evict random entries if we exceed capacity significantly
        // For now, rely on DashMap's internal handling
    }

    pub fn probe(&self, hash: u64, depth: u8, alpha: i16, beta: i16) -> Option<(i16, Option<M>)> {
        // Lock-free read with DashMap
        if let Some(entry) = self.table.get(&hash) {
            if entry.depth >= depth {
                match entry.bound_type {
                    BoundType::Exact => {
                        self.hits.fetch_add(1, Ordering::Relaxed);
                        return Some((entry.score, entry.best_move.clone()));
                    }
                    BoundType::Lower if entry.score >= beta => {
                        self.hits.fetch_add(1, Ordering::Relaxed);
                        return Some((beta, entry.best_move.clone()));
                    }
                    BoundType::Upper if entry.score <= alpha => {
                        self.hits.fetch_add(1, Ordering::Relaxed);
                        return Some((alpha, entry.best_move.clone()));
                    }
                    _ => (),
                }
            }
        }
        None
    }

    pub fn get_move(&self, hash: u64) -> Option<M> {
        // Lock-free read
        self.table
            .get(&hash)
            .and_then(|entry| entry.best_move.clone())
    }

    /// Probe TT and return both cutoff score (if applicable) and best move (if exists).
    /// Returns (Some(score), best_move) if early cutoff possible, (None, best_move) otherwise.
    /// This is more efficient than calling probe() followed by get_move().
    pub fn probe_with_move(
        &self,
        hash: u64,
        depth: u8,
        alpha: i16,
        beta: i16,
    ) -> (Option<i16>, Option<M>) {
        // Lock-free read with DashMap
        if let Some(entry) = self.table.get(&hash) {
            let best_move = entry.best_move.clone();

            if entry.depth >= depth {
                match entry.bound_type {
                    BoundType::Exact => {
                        self.hits.fetch_add(1, Ordering::Relaxed);
                        return (Some(entry.score), best_move);
                    }
                    BoundType::Lower if entry.score >= beta => {
                        self.hits.fetch_add(1, Ordering::Relaxed);
                        return (Some(beta), best_move);
                    }
                    BoundType::Upper if entry.score <= alpha => {
                        self.hits.fetch_add(1, Ordering::Relaxed);
                        return (Some(alpha), best_move);
                    }
                    _ => (),
                }
            }
            // Entry exists but doesn't allow cutoff - return move for ordering
            (None, best_move)
        } else {
            // No entry found
            (None, None)
        }
    }

    pub fn clear(&self) {
        self.table.clear();
        self.hits.store(0, Ordering::Relaxed);
    }

    pub fn hits(&self) -> usize {
        self.hits.load(Ordering::Relaxed)
    }
}
