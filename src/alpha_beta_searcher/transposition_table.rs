use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::RwLock;

use crate::chess_move::chess_move::ChessMove;

// Each entry represents a previously searched position
#[derive(Clone)]
pub struct TTEntry {
    // The evaluated score for this position
    pub score: i16,
    // How deep we searched when we got this score
    pub depth: u8,
    // What kind of score this is
    pub bound_type: BoundType,
    // The best move we found in this position
    pub best_move: Option<ChessMove>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum BoundType {
    // We know the exact score
    Exact,
    // We know the score is at least this high
    Lower,
    // We know the score is at most this high
    Upper,
}

const DEFAULT_TT_SIZE_MB: usize = 64;

pub struct TranspositionTable {
    // LruCache automatically maintains a fixed-size cache with least-recently-used eviction
    table: RwLock<LruCache<u64, TTEntry>>,
    // Track how often we successfully use cached results
    hits: AtomicUsize,
}

impl Default for TranspositionTable {
    fn default() -> Self {
        Self::new(DEFAULT_TT_SIZE_MB)
    }
}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
        // Calculate number of entries that will fit in size_mb megabytes
        // Each entry uses ~32 bytes (score: 2, depth: 1, bound_type: 1,
        // best_move: ~24, plus overhead)
        let entry_size = 32;
        let num_entries = (size_mb * 1024 * 1024) / entry_size;

        Self {
            table: RwLock::new(LruCache::new(NonZeroUsize::new(num_entries).unwrap())),
            hits: AtomicUsize::new(0),
        }
    }

    pub fn store(
        &self,
        hash: u64,
        score: i16,
        depth: u8,
        bound_type: BoundType,
        best_move: Option<ChessMove>,
    ) {
        let entry = TTEntry {
            score,
            depth,
            bound_type,
            best_move,
        };

        let mut table = self.table.write().unwrap();
        table.put(hash, entry);
    }

    pub fn probe(
        &self,
        hash: u64,
        depth: u8,
        alpha: i16,
        beta: i16,
    ) -> Option<(i16, Option<ChessMove>)> {
        let mut table = self.table.write().unwrap();

        if let Some(entry) = table.get(&hash) {
            // Only use entries from searches at least as deep as current
            if entry.depth >= depth {
                match entry.bound_type {
                    // For exact scores, we can use them directly
                    BoundType::Exact => {
                        self.hits.fetch_add(1, Ordering::Relaxed);
                        return Some((entry.score, entry.best_move.clone()));
                    }
                    // For lower bounds, if the score beats beta we can use it
                    BoundType::Lower if entry.score >= beta => {
                        self.hits.fetch_add(1, Ordering::Relaxed);
                        return Some((beta, entry.best_move.clone()));
                    }
                    // For upper bounds, if the score is below alpha we can use it
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

    pub fn clear(&self) {
        let mut table = self.table.write().unwrap();
        table.clear();
        self.hits.store(0, Ordering::Relaxed);
    }
}
