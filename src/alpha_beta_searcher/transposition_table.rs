//! Generic transposition table for caching search results.
//!
//! Uses a fixed-size array indexed by hash % capacity with two slots per bucket:
//! one depth-preferred (only replaced by equal or deeper entries) and one always-replace.
//! This bounded-memory design provides much better cache locality than a hash map.

use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone)]
pub struct TTEntry<M: Clone> {
    pub key: u32,
    pub score: i16,
    pub depth: u8,
    pub bound_type: BoundType,
    pub best_move: Option<M>,
}

impl<M: Clone> Default for TTEntry<M> {
    fn default() -> Self {
        Self {
            key: 0,
            score: 0,
            depth: 0,
            bound_type: BoundType::Upper,
            best_move: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BoundType {
    Exact,
    Lower,
    Upper,
}

/// A two-slot bucket: slot 0 is depth-preferred, slot 1 is always-replace.
#[derive(Clone)]
struct TTBucket<M: Clone> {
    depth_slot: TTEntry<M>,
    replace_slot: TTEntry<M>,
}

impl<M: Clone> Default for TTBucket<M> {
    fn default() -> Self {
        Self {
            depth_slot: TTEntry::default(),
            replace_slot: TTEntry::default(),
        }
    }
}

const DEFAULT_TT_SIZE_MB: usize = 64;

/// Fixed-size transposition table with bounded memory and O(1) access.
///
/// Uses `UnsafeCell` for lock-free concurrent access. Benign data races are
/// acceptable (may read a partially-written entry), which is the standard
/// approach in chess engines. The hash verification key catches most corruption.
pub struct TranspositionTable<M: Clone + Send + Sync> {
    // UnsafeCell allows concurrent mutable access without locks.
    // Safety: benign races are acceptable -- the verification key catches corruption.
    table: Vec<std::cell::UnsafeCell<TTBucket<M>>>,
    capacity: usize,
    hits: AtomicUsize,
    depth_rejected: AtomicUsize,
    bound_rejected: AtomicUsize,
    overwrites: AtomicUsize,
}

// Safety: concurrent access produces benign races (stale reads, partial writes)
// that are detected by the key verification check. This is the standard approach
// for transposition tables in chess engines.
unsafe impl<M: Clone + Send + Sync> Sync for TranspositionTable<M> {}

impl<M: Clone + Send + Sync> Default for TranspositionTable<M> {
    fn default() -> Self {
        Self::new(DEFAULT_TT_SIZE_MB)
    }
}

impl<M: Clone + Send + Sync> TranspositionTable<M> {
    pub fn new(size_mb: usize) -> Self {
        let bucket_size = std::mem::size_of::<TTBucket<M>>().max(1);
        let num_buckets = (size_mb * 1024 * 1024) / bucket_size;
        // Use power of 2 for fast modulo via bitwise AND
        let capacity = num_buckets.next_power_of_two();

        let mut table = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            table.push(std::cell::UnsafeCell::new(TTBucket::default()));
        }

        Self {
            table,
            capacity,
            hits: AtomicUsize::new(0),
            depth_rejected: AtomicUsize::new(0),
            bound_rejected: AtomicUsize::new(0),
            overwrites: AtomicUsize::new(0),
        }
    }

    #[inline]
    fn index(&self, hash: u64) -> usize {
        (hash as usize) & (self.capacity - 1)
    }

    #[inline]
    fn verification_key(hash: u64) -> u32 {
        // Ensure key is never 0 (reserved for empty entries)
        let key = (hash >> 32) as u32;
        if key == 0 {
            1
        } else {
            key
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
        let idx = self.index(hash);
        let key = Self::verification_key(hash);

        let entry = TTEntry {
            key,
            score,
            depth,
            bound_type,
            best_move,
        };

        // Safety: benign races are acceptable -- verified by key check on read.
        let bucket = unsafe { &mut *self.table[idx].get() };

        // Depth-preferred slot: only replace if new entry has >= depth
        if depth >= bucket.depth_slot.depth || bucket.depth_slot.key == 0 {
            if bucket.depth_slot.key != 0 {
                self.overwrites.fetch_add(1, Ordering::Relaxed);
            }
            bucket.depth_slot = entry;
        } else {
            // Always-replace slot: always overwrite
            bucket.replace_slot = entry;
        }
    }

    /// Probe TT and return both cutoff score (if applicable) and best move (if exists).
    /// Returns (Some(score), best_move) if early cutoff possible, (None, best_move)
    /// otherwise.
    pub fn probe_with_move(
        &self,
        hash: u64,
        depth: u8,
        alpha: i16,
        beta: i16,
    ) -> (Option<i16>, Option<M>) {
        let idx = self.index(hash);
        let key = Self::verification_key(hash);

        // Safety: benign races -- verified by key check.
        let bucket = unsafe { &*self.table[idx].get() };

        // Check depth-preferred slot first (more likely to be useful)
        if bucket.depth_slot.key == key {
            return self.check_entry(&bucket.depth_slot, depth, alpha, beta);
        }

        // Check always-replace slot
        if bucket.replace_slot.key == key {
            return self.check_entry(&bucket.replace_slot, depth, alpha, beta);
        }

        (None, None)
    }

    #[inline]
    fn check_entry(
        &self,
        entry: &TTEntry<M>,
        depth: u8,
        alpha: i16,
        beta: i16,
    ) -> (Option<i16>, Option<M>) {
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
                _ => {
                    self.bound_rejected.fetch_add(1, Ordering::Relaxed);
                }
            }
        } else {
            self.depth_rejected.fetch_add(1, Ordering::Relaxed);
        }

        // Entry exists but doesn't allow cutoff -- return move for ordering
        (None, best_move)
    }

    pub fn clear(&self) {
        for cell in &self.table {
            // Safety: called between searches, no concurrent access expected.
            let bucket = unsafe { &mut *cell.get() };
            *bucket = TTBucket::default();
        }
        self.hits.store(0, Ordering::Relaxed);
        self.depth_rejected.store(0, Ordering::Relaxed);
        self.bound_rejected.store(0, Ordering::Relaxed);
        self.overwrites.store(0, Ordering::Relaxed);
    }

    pub fn hits(&self) -> usize {
        self.hits.load(Ordering::Relaxed)
    }

    pub fn size(&self) -> usize {
        // Count non-empty entries
        let mut count = 0;
        for cell in &self.table {
            let bucket = unsafe { &*cell.get() };
            if bucket.depth_slot.key != 0 {
                count += 1;
            }
            if bucket.replace_slot.key != 0 {
                count += 1;
            }
        }
        count
    }

    pub fn depth_rejected(&self) -> usize {
        self.depth_rejected.load(Ordering::Relaxed)
    }

    pub fn bound_rejected(&self) -> usize {
        self.bound_rejected.load(Ordering::Relaxed)
    }

    pub fn overwrites(&self) -> usize {
        self.overwrites.load(Ordering::Relaxed)
    }
}
