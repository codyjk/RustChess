//! Memory profiling instrumentation for tracking allocations and operations.

use std::sync::atomic::{AtomicUsize, Ordering};

pub struct MemoryProfiler;

static BOARD_CLONES: AtomicUsize = AtomicUsize::new(0);
static MOVEGEN_CREATES: AtomicUsize = AtomicUsize::new(0);

impl MemoryProfiler {
    pub fn record_board_clone() {
        BOARD_CLONES.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_movegen_create() {
        MOVEGEN_CREATES.fetch_add(1, Ordering::Relaxed);
    }

    pub fn print_stats() {
        println!("Board clones: {}", BOARD_CLONES.load(Ordering::Relaxed));
        println!(
            "MoveGen creates: {}",
            MOVEGEN_CREATES.load(Ordering::Relaxed)
        );
    }

    pub fn reset() {
        BOARD_CLONES.store(0, Ordering::Relaxed);
        MOVEGEN_CREATES.store(0, Ordering::Relaxed);
    }
}
