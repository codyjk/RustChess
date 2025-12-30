//! History heuristic for chess move ordering.
//!
//! Tracks which quiet moves cause beta cutoffs, using this information to improve
//! move ordering. Moves that frequently cause cutoffs are prioritized over moves
//! that rarely do.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::prelude::*;

const HISTORY_SIZE: usize = 64 * 64; // from_square * 64 + to_square

/// Thread-local history table tracking move success rates.
///
/// Uses atomic operations for thread-safety in parallel search. Each entry
/// stores a counter that increases when a move causes a beta cutoff.
pub struct HistoryTable {
    table: Vec<AtomicU32>,
}

impl HistoryTable {
    pub fn new() -> Self {
        Self {
            table: (0..HISTORY_SIZE).map(|_| AtomicU32::new(0)).collect(),
        }
    }

    #[inline]
    fn index(from: Square, to: Square) -> usize {
        (from.index() as usize) * 64 + (to.index() as usize)
    }

    /// Records that a move from `from` to `to` caused a beta cutoff.
    pub fn record_cutoff(&self, from: Square, to: Square, depth: u8) {
        let idx = Self::index(from, to);
        // Weight by depth: deeper cutoffs are more significant
        let bonus = (depth as u32 + 1) * (depth as u32 + 1);
        self.table[idx].fetch_add(bonus, Ordering::Relaxed);
    }

    /// Returns the history score for a move from `from` to `to`.
    #[inline]
    pub fn score(&self, from: Square, to: Square) -> u32 {
        let idx = Self::index(from, to);
        self.table[idx].load(Ordering::Relaxed)
    }

    /// Ages all entries by dividing by 2, preventing unbounded growth.
    pub fn age(&self) {
        for entry in self.table.iter() {
            let current = entry.load(Ordering::Relaxed);
            entry.store(current / 2, Ordering::Relaxed);
        }
    }

    /// Clears all history entries.
    pub fn clear(&self) {
        for entry in self.table.iter() {
            entry.store(0, Ordering::Relaxed);
        }
    }
}

impl Default for HistoryTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_recording() {
        let history = HistoryTable::new();
        let from = Square::E2;
        let to = Square::E4;

        assert_eq!(history.score(from, to), 0);

        history.record_cutoff(from, to, 3);
        assert!(history.score(from, to) > 0);

        let score1 = history.score(from, to);
        history.record_cutoff(from, to, 4);
        let score2 = history.score(from, to);
        assert!(score2 > score1);
    }

    #[test]
    fn test_history_aging() {
        let history = HistoryTable::new();
        let from = Square::E2;
        let to = Square::E4;

        history.record_cutoff(from, to, 5);
        let score_before = history.score(from, to);
        assert!(score_before > 0);

        history.age();
        let score_after = history.score(from, to);
        assert_eq!(score_after, score_before / 2);
    }

    #[test]
    fn test_history_clear() {
        let history = HistoryTable::new();
        let from = Square::E2;
        let to = Square::E4;

        history.record_cutoff(from, to, 3);
        assert!(history.score(from, to) > 0);

        history.clear();
        assert_eq!(history.score(from, to), 0);
    }

    #[test]
    fn test_history_different_moves() {
        let history = HistoryTable::new();

        history.record_cutoff(Square::E2, Square::E4, 3);
        history.record_cutoff(Square::D2, Square::D4, 4);

        assert!(history.score(Square::E2, Square::E4) > 0);
        assert!(history.score(Square::D2, Square::D4) > 0);
        // Depth 4 gives higher bonus than depth 3 (25 vs 16)
        assert!(history.score(Square::D2, Square::D4) > history.score(Square::E2, Square::E4));
    }
}
