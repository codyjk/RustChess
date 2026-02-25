//! Quick alpha-beta performance benchmark for fast iteration.

use std::str::FromStr;
use std::time::{Duration, Instant};

use crate::alpha_beta_searcher::SearchContext;
use crate::board::Board;
use crate::chess_search::search_best_move;
use crate::diagnostics::memory_profiler::MemoryProfiler;

/// A test position with metadata for benchmarking.
struct BenchmarkPosition {
    name: &'static str,
    fen: &'static str,
}

impl BenchmarkPosition {
    fn board(&self) -> Board {
        Board::from_str(self.fen).expect("benchmark FEN should be valid")
    }
}

/// Curated set of positions representing different search characteristics.
const BENCHMARK_POSITIONS: &[BenchmarkPosition] = &[
    BenchmarkPosition {
        name: "Starting Position",
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    },
    BenchmarkPosition {
        name: "Sicilian Defense",
        fen: "rnbqkbnr/pp1ppppp/8/2p5/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2",
    },
    BenchmarkPosition {
        name: "Middlegame Tactics",
        fen: "r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
    },
    BenchmarkPosition {
        name: "Tactical Position",
        fen: "r1bqk2r/pppp1ppp/2n2n2/2b1p3/2B1P3/3P1N2/PPP2PPP/RNBQK2R w KQkq - 0 5",
    },
    BenchmarkPosition {
        name: "King Safety",
        fen: "rnbqk2r/ppp2ppp/3b1n2/3pp3/3PP3/3B1N2/PPP2PPP/RNBQK2R w KQkq - 0 6",
    },
    BenchmarkPosition {
        name: "Endgame Pattern",
        fen: "4k3/8/8/8/8/8/4P3/4K3 w - - 0 1",
    },
    BenchmarkPosition {
        name: "Queen Endgame",
        fen: "4k3/8/8/8/8/8/8/4K2Q w - - 0 1",
    },
    BenchmarkPosition {
        name: "Rook Endgame",
        fen: "4k3/8/8/8/8/8/R7/4K3 w - - 0 1",
    },
];

struct PositionResult {
    position_name: String,
    best_move: String,
    score: i16,
    nodes_searched: usize,
    time_taken: Duration,
}

impl PositionResult {
    fn nodes_per_second(&self) -> f64 {
        self.nodes_searched as f64 / self.time_taken.as_secs_f64()
    }

    fn print(&self) {
        println!("\nPosition: {}", self.position_name);
        println!("  Best move: {} (score: {:+})", self.best_move, self.score);
        println!(
            "  Nodes: {:>12} | Time: {:>6.2}s | Speed: {:>8.0}k nodes/s",
            format_number(self.nodes_searched),
            self.time_taken.as_secs_f64(),
            self.nodes_per_second() / 1000.0
        );
    }
}

struct BenchmarkSummary {
    total_nodes: usize,
    total_quiescence_nodes: usize,
    total_time: Duration,
    total_tt_hits: usize,
    total_tt_probes: usize,
    total_tt_stores: usize,
    total_tt_misses: usize,
    total_tt_depth_rejected: usize,
    total_tt_bound_rejected: usize,
    total_tt_overwrites: usize,
    tt_final_size: usize,
    total_move_gen_calls: usize,
    total_null_move_attempts: usize,
    total_null_move_cutoffs: usize,
    total_rfp_attempts: usize,
    total_rfp_cutoffs: usize,
    total_fp_attempts: usize,
    total_fp_cutoffs: usize,
    total_check_extensions: usize,
    results: Vec<PositionResult>,
}

impl BenchmarkSummary {
    fn average_nodes_per_second(&self) -> f64 {
        self.total_nodes as f64 / self.total_time.as_secs_f64()
    }

    fn tt_hit_rate(&self) -> f64 {
        if self.total_tt_probes == 0 {
            0.0
        } else {
            (self.total_tt_hits as f64 / self.total_tt_probes as f64) * 100.0
        }
    }

    fn move_gen_per_node(&self) -> f64 {
        if self.total_nodes == 0 {
            0.0
        } else {
            self.total_move_gen_calls as f64 / self.total_nodes as f64
        }
    }

    fn print(&self, depth: u8, parallel: bool) {
        println!("\n{}", "=".repeat(70));
        println!(
            "Alpha-Beta Performance Benchmark (depth: {}, parallel: {})",
            depth, parallel
        );
        println!("{}", "=".repeat(70));

        for result in &self.results {
            result.print();
        }

        println!("\n{}", "=".repeat(70));
        println!("SUMMARY");
        println!("{}", "-".repeat(70));
        println!("  Total nodes:    {:>12}", format_number(self.total_nodes));
        println!(
            "    Quiescence:   {:>12} ({:.1}% of nodes)",
            format_number(self.total_quiescence_nodes),
            (self.total_quiescence_nodes as f64 / self.total_nodes as f64) * 100.0
        );
        println!("  Total time:     {:>12.2}s", self.total_time.as_secs_f64());
        println!(
            "  Avg speed:      {:>12.0}k nodes/s",
            self.average_nodes_per_second() / 1000.0
        );
        println!();
        println!("  Transposition Table:");
        println!(
            "    Probes:       {:>12} ({:.2} per node)",
            format_number(self.total_tt_probes),
            self.total_tt_probes as f64 / self.total_nodes as f64
        );
        println!(
            "    Hits:         {:>12} ({:.1}% of probes)",
            format_number(self.total_tt_hits),
            self.tt_hit_rate()
        );
        println!(
            "    Misses:       {:>12} ({:.1}% of probes)",
            format_number(self.total_tt_misses),
            (self.total_tt_misses as f64 / self.total_tt_probes as f64) * 100.0
        );
        println!(
            "    Stores:       {:>12} ({:.2} per node)",
            format_number(self.total_tt_stores),
            self.total_tt_stores as f64 / self.total_nodes as f64
        );
        println!(
            "    Depth reject: {:>12} ({:.1}% of probes)",
            format_number(self.total_tt_depth_rejected),
            (self.total_tt_depth_rejected as f64 / self.total_tt_probes as f64) * 100.0
        );
        println!(
            "    Bound reject: {:>12} ({:.1}% of probes)",
            format_number(self.total_tt_bound_rejected),
            (self.total_tt_bound_rejected as f64 / self.total_tt_probes as f64) * 100.0
        );
        println!(
            "    Overwrites:   {:>12} ({:.1}% of stores)",
            format_number(self.total_tt_overwrites),
            (self.total_tt_overwrites as f64 / self.total_tt_stores as f64) * 100.0
        );
        println!(
            "    Final size:   {:>12} entries",
            format_number(self.tt_final_size)
        );
        println!();
        println!(
            "  Move gen calls: {:>12} ({:.2} per node)",
            format_number(self.total_move_gen_calls),
            self.move_gen_per_node()
        );
        println!();
        println!("  Null Move Pruning:");
        println!(
            "    Attempts:     {:>12}",
            format_number(self.total_null_move_attempts)
        );
        let cutoff_pct = if self.total_null_move_attempts == 0 {
            0.0
        } else {
            (self.total_null_move_cutoffs as f64 / self.total_null_move_attempts as f64) * 100.0
        };
        println!(
            "    Cutoffs:      {:>12} ({:.1}% of attempts)",
            format_number(self.total_null_move_cutoffs),
            cutoff_pct
        );
        println!();
        println!("  Reverse Futility Pruning:");
        println!(
            "    Attempts:     {:>12}",
            format_number(self.total_rfp_attempts)
        );
        let rfp_cutoff_pct = if self.total_rfp_attempts == 0 {
            0.0
        } else {
            (self.total_rfp_cutoffs as f64 / self.total_rfp_attempts as f64) * 100.0
        };
        println!(
            "    Cutoffs:      {:>12} ({:.1}% of attempts)",
            format_number(self.total_rfp_cutoffs),
            rfp_cutoff_pct
        );
        println!();
        println!("  Futility Pruning:");
        println!(
            "    Attempts:     {:>12}",
            format_number(self.total_fp_attempts)
        );
        let fp_cutoff_pct = if self.total_fp_attempts == 0 {
            0.0
        } else {
            (self.total_fp_cutoffs as f64 / self.total_fp_attempts as f64) * 100.0
        };
        println!(
            "    Cutoffs:      {:>12} ({:.1}% of attempts)",
            format_number(self.total_fp_cutoffs),
            fp_cutoff_pct
        );
        println!();
        println!(
            "  Check extensions: {:>10}",
            format_number(self.total_check_extensions)
        );
        println!("{}", "=".repeat(70));
    }
}

fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// List all available benchmark positions.
pub fn list_positions() {
    for (i, pos) in BENCHMARK_POSITIONS.iter().enumerate() {
        println!("  [{}] {}", i, pos.name);
    }
}

/// Run the alpha-beta benchmark on a curated set of positions.
///
/// If `position_filter` is provided, only positions matching the filter will be run.
/// The filter can be:
/// - An index (e.g., "0", "3")
/// - A name substring (case-insensitive, e.g., "endgame", "sicilian")
pub fn run_alpha_beta_benchmark(depth: u8, parallel: bool, position_filter: Option<String>) {
    MemoryProfiler::reset();

    // Filter positions if requested
    let positions_to_run: Vec<&BenchmarkPosition> = if let Some(ref filter) = position_filter {
        // Try to parse as index first
        if let Ok(index) = filter.parse::<usize>() {
            if index < BENCHMARK_POSITIONS.len() {
                vec![&BENCHMARK_POSITIONS[index]]
            } else {
                eprintln!(
                    "Error: Position index {} is out of range (0-{})",
                    index,
                    BENCHMARK_POSITIONS.len() - 1
                );
                eprintln!("\nAvailable positions:");
                list_positions();
                return;
            }
        } else {
            // Filter by name substring (case-insensitive)
            let filter_lower = filter.to_lowercase();
            let filtered: Vec<&BenchmarkPosition> = BENCHMARK_POSITIONS
                .iter()
                .filter(|pos| pos.name.to_lowercase().contains(&filter_lower))
                .collect();

            if filtered.is_empty() {
                eprintln!("Error: No positions match filter '{}'", filter);
                eprintln!("\nAvailable positions:");
                list_positions();
                return;
            }
            filtered
        }
    } else {
        BENCHMARK_POSITIONS.iter().collect()
    };

    let mut results = Vec::new();
    let mut total_nodes = 0;
    let mut total_quiescence_nodes = 0;
    let mut total_time = Duration::from_secs(0);
    let mut total_tt_probes = 0;
    let mut total_tt_stores = 0;
    let mut total_tt_misses = 0;
    let mut total_move_gen_calls = 0;
    let mut total_null_move_attempts = 0;
    let mut total_null_move_cutoffs = 0;
    let mut total_rfp_attempts = 0;
    let mut total_rfp_cutoffs = 0;
    let mut total_fp_attempts = 0;
    let mut total_fp_cutoffs = 0;
    let mut total_check_extensions = 0;

    // Create SearchContext once and share TT across all positions
    let mut context = SearchContext::with_parallel(depth, parallel);

    for benchmark_pos in positions_to_run {
        let mut board = benchmark_pos.board();

        // Reset stats but keep TT entries for cross-position transpositions
        context.reset_stats_keep_tt();

        let start = Instant::now();
        let best_move = search_best_move(&mut context, &mut board)
            .expect("search should find a move in benchmark position");
        let time_taken = start.elapsed();

        let nodes_searched = context.searched_position_count();
        let quiescence_nodes = context.quiescence_nodes();
        let score = context.last_score().unwrap_or(0);
        let tt_probes = context.tt_probes();
        let tt_stores = context.tt_stores();
        let tt_misses = context.tt_probe_misses();
        let move_gen_calls = context.move_gen_calls();
        let null_move_attempts = context.null_move_attempts();
        let null_move_cutoffs = context.null_move_cutoffs();
        let rfp_attempts = context.rfp_attempts();
        let rfp_cutoffs = context.rfp_cutoffs();
        let fp_attempts = context.fp_attempts();
        let fp_cutoffs = context.fp_cutoffs();
        let check_extensions = context.check_extension_count();

        total_nodes += nodes_searched;
        total_quiescence_nodes += quiescence_nodes;
        total_time += time_taken;
        total_tt_probes += tt_probes;
        total_tt_stores += tt_stores;
        total_tt_misses += tt_misses;
        total_move_gen_calls += move_gen_calls;
        total_null_move_attempts += null_move_attempts;
        total_null_move_cutoffs += null_move_cutoffs;
        total_rfp_attempts += rfp_attempts;
        total_rfp_cutoffs += rfp_cutoffs;
        total_fp_attempts += fp_attempts;
        total_fp_cutoffs += fp_cutoffs;
        total_check_extensions += check_extensions;

        results.push(PositionResult {
            position_name: benchmark_pos.name.to_string(),
            best_move: best_move.to_string(),
            score,
            nodes_searched,
            time_taken,
        });
    }

    // Read TT counters once at the end (they accumulate across positions)
    let total_tt_hits = context.tt_hits();
    let total_tt_depth_rejected = context.tt_depth_rejected();
    let total_tt_bound_rejected = context.tt_bound_rejected();
    let total_tt_overwrites = context.tt_overwrites();
    let tt_final_size = context.tt_size();

    let summary = BenchmarkSummary {
        total_nodes,
        total_quiescence_nodes,
        total_time,
        total_tt_hits,
        total_tt_probes,
        total_tt_stores,
        total_tt_misses,
        total_tt_depth_rejected,
        total_tt_bound_rejected,
        total_tt_overwrites,
        tt_final_size,
        total_move_gen_calls,
        total_null_move_attempts,
        total_null_move_cutoffs,
        total_rfp_attempts,
        total_rfp_cutoffs,
        total_fp_attempts,
        total_fp_cutoffs,
        total_check_extensions,
        results,
    };

    summary.print(depth, parallel);

    println!();
    MemoryProfiler::print_stats();
}
