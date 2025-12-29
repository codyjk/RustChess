//! Generic transposition table for caching search results.

use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::RwLock;

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
    table: RwLock<LruCache<u64, TTEntry<M>>>,
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
            table: RwLock::new(LruCache::new(
                NonZeroUsize::new(num_entries).expect("num_entries should be non-zero"),
            )),
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

        let mut table = self
            .table
            .write()
            .expect("transposition table lock should not be poisoned");
        table.put(hash, entry);
    }

    pub fn probe(&self, hash: u64, depth: u8, alpha: i16, beta: i16) -> Option<(i16, Option<M>)> {
        let mut table = self
            .table
            .write()
            .expect("transposition table lock should not be poisoned");

        if let Some(entry) = table.get(&hash) {
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

    pub fn clear(&self) {
        let mut table = self
            .table
            .write()
            .expect("transposition table lock should not be poisoned");
        table.clear();
        self.hits.store(0, Ordering::Relaxed);
    }

    pub fn hits(&self) -> usize {
        self.hits.load(Ordering::Relaxed)
    }
}
