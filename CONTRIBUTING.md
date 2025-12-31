# Contributing

This guide provides information for developers working on the chess engine codebase, including development setup, architecture, code standards, and optimization workflows.

## Development Setup

### Building and Running

```bash
# Build release version (required for performance testing)
cargo build --release

# Install the binary
cargo install --path .

# Run the engine
chess --help
```

**IMPORTANT**: Always use the installed `chess` binary (available via `which chess`) rather than `./target/release/chess`. The binary is installed at `~/.cargo/bin/chess` and should be in your PATH.

### Testing and Quality

```bash
# Run all tests
cargo test

# Run benchmarks (requires release profile)
cargo bench

# Format code (required before commit)
cargo fmt

# Lint code (required before commit)
cargo clippy -- -D warnings

# Profile with flamegraph
sudo cargo flamegraph --bench pvp_benchmark
```

### Common Subcommands

```bash
# Play against the engine
chess play --depth 4 --color white

# Calculate best move from FEN position
chess calculate-best-move --fen "1Q6/8/8/8/8/k1K5/8/8 w - - 0 1"

# Count positions (performance test)
chess count-positions --depth 6 --strategy alpha-beta

# Test against Stockfish
chess determine-stockfish-elo --depth 6 --starting-elo 2000
```

## Architecture

### Core Design Philosophy

The engine separates concerns into three layers:

1. **Generic alpha-beta search** (`alpha_beta_searcher/`) - Game-agnostic algorithm using traits
2. **Chess-specific implementations** (`chess_search/`) - Chess logic implementing the search traits
3. **Board representation & move generation** (`board/`, `move_generator/`) - Low-level chess mechanics

### Key Architectural Patterns

**Trait-Based Search**: The alpha-beta algorithm is implemented generically using traits (`MoveGenerator`, `Evaluator`, `MoveOrderer`). Chess-specific implementations live in `chess_search/implementation.rs`. This separation enables:
- Independent testing of search algorithm logic
- Clean separation between algorithm and domain logic
- Type-safe abstraction boundaries

**Bitboard Representation**: Board state uses 64-bit integers (`Bitboard`) for efficient bitwise operations. Individual squares are represented as newtype-wrapped u8 indices (`Square`). This enables CPU-level optimizations for move generation and attack calculations.

**Move Type Hierarchy**: Chess moves are represented as an enum (`ChessMove`) with variants for different move types:
- `StandardChessMove` - Regular piece moves and captures
- `CastleChessMove` - King and rook castling
- `EnPassantChessMove` - Special pawn capture
- `PawnPromotionChessMove` - Pawn reaching back rank

Each variant implements the `ChessMoveEffect` trait for applying/unapplying moves to board state.

**State Management**: Board maintains a `StateStack` for efficient undo operations. Move application pushes state, move reversal pops state. This enables the search algorithm to explore game trees without expensive cloning.

**Zobrist Hashing**: Hash tables generated at compile time by `precompile/` build script enable incremental hashing for caching move generation and transposition table lookups.

**Move Ordering**: The `ChessMoveOrderer` prioritizes moves to improve alpha-beta pruning efficiency:
1. Captures (MVV-LVA ordering)
2. Killer moves
3. Other moves

Better ordering = more pruning = faster search.

### Module Organization

**`common/`** - Shared types between engine and precompiler
- `bitboard/bitboard.rs` - 64-bit integer set operations
- `bitboard/square.rs` - Newtype-wrapped u8 for board squares

**`precompile/`** - Build-time code generation (runs at compile time)
- `zobrist/` - Generates Zobrist hash tables
- `magic/` - Computes magic bitboard numbers for sliding piece attacks
- `book/` - Generates opening book from PGN data

**`src/`** - Main engine implementation
- `prelude.rs` - Common type re-exports (`Board`, `Color`, `Piece`, `ChessMove`, `Bitboard`, `Square`)
- `alpha_beta_searcher/` - Generic search algorithm with traits
  - `search.rs` - Core alpha-beta implementation with iterative deepening and quiescence search
  - `traits.rs` - Game-agnostic trait definitions (`MoveGenerator`, `Evaluator`, `MoveOrderer`)
  - `transposition_table.rs` - LRU cache for position caching
  - `killer_moves.rs` - Thread-local killer move storage for parallel search
  - `tests.rs` - Comprehensive search algorithm tests
- `chess_search/` - Chess-specific search implementations
  - `implementation.rs` - Implements `MoveGenerator`, `Evaluator` traits for chess
  - `move_orderer.rs` - Move ordering heuristics (MVV-LVA, killer moves, PV)
  - `history_table.rs` - History heuristic for move ordering
  - `tests.rs` - Chess search tests
- `board/` - Board state representation
  - `board.rs` - Main `Board` struct with piece placement and game state
  - `piece_set.rs` - Per-color piece bitboards
  - `state_stack.rs` - Undo stack for move reversal
  - `color.rs`, `piece.rs` - Color and piece type definitions
  - `castle_rights.rs`, `halfmove_clock.rs`, `fullmove_number.rs` - Game state newtypes
  - `display.rs`, `error.rs`, `move_info.rs`, `position_info.rs` - Board utilities
  - `tests.rs` - Board state tests
- `chess_move/` - Move types and application
  - `chess_move.rs` - `ChessMove` enum with all move variants
  - `chess_move_effect.rs` - Trait for applying/unapplying moves to board
  - `standard.rs`, `castle.rs`, `en_passant.rs`, `pawn_promotion.rs`, `capture.rs` - Move implementations
  - `algebraic_notation.rs` - Display moves in standard notation
  - `traits.rs` - Move-related traits
- `move_generator/` - Legal move generation
  - `generator.rs` - Main move generation logic with caching
  - `targets.rs` - Calculate attack/movement targets for pieces
  - `magic_table.rs` - Magic bitboards for sliding pieces (rooks, bishops, queens)
- `evaluate/` - Position scoring
  - `evaluation.rs` - Board evaluation heuristic (material + position)
  - `evaluation_tables.rs` - Piece-square tables for positional scoring
- `game/` - Game loop and I/O
  - `loop.rs` - Main game loop using `InputSource` and `GameRenderer` traits
  - `engine.rs` - Computer player using alpha-beta search
  - `input_source.rs`, `renderer.rs` - Trait abstractions for I/O
  - `display.rs`, `action.rs`, `mode.rs`, `util.rs` - Game utilities
  - `stockfish_interface.rs`, `stockfish_elo.rs` - Stockfish integration for testing
  - `position_counter.rs`, `alpha_beta_benchmark.rs` - Performance testing utilities
- `book/` - Opening book
  - `book.rs` - Opening book lookup for move suggestions
- `input_handler/` - Input parsing
  - `fen.rs`, `fen_serialize.rs` - FEN notation parsing and serialization
  - `input.rs` - User input handling
- `cli/` - Command-line interface
  - `args.rs` - Argument parsing
  - `commands/` - Subcommand implementations (play, uci, benchmark, etc.)
- `uci/` - UCI protocol implementation
  - `protocol.rs` - UCI protocol state machine
  - `command_parser.rs` - UCI command parsing
  - `response_formatter.rs` - UCI response formatting
- `tui/` - Terminal user interface
  - `app.rs` - Main TUI application state and rendering
  - `board_widget.rs` - Chess board widget for ratatui
  - `theme.rs` - Color theme management
- `diagnostics/` - Performance diagnostics
  - `memory_profiler.rs` - Memory allocation tracking and profiling

### Important Implementation Details

**`chess_position!` Macro**: Instantiates board state from ASCII art. Used extensively in tests:
```rust
let board = chess_position! {
    rnbqkbnr
    pppppppp
    ........
    ........
    ........
    ........
    PPPPPPPP
    RNBQKBNR
};
```

**Build Script**: `Cargo.toml` specifies `build = "precompile/src/main.rs"`. This runs the precompiler before compilation to generate Zobrist tables and magic numbers.

**Release Profile Optimizations**: Critical for performance testing (see `Cargo.toml`):
- `lto = true` - Link-time optimization
- `codegen-units = 1` - Single codegen unit for better optimization
- `opt-level = 3` - Maximum optimization

## Code Standards

### Module Structure

The codebase follows idiomatic Rust module organization practices:

- **`mod.rs` files**: Only module declarations and re-exports, no implementation
- **Implementation files**: Dedicated `.rs` files (e.g., `board/board.rs`, not in `mod.rs`)
- **Module docstrings**: Required `//!` comment at top of all module and implementation files

### Import Organization

All imports follow a consistent ordering standard (enforced by the pre-commit hook):

1. Standard library (`std::`, `core::`)
2. External crates (`common`, `rayon`, `smallvec`, etc.)
3. Crate imports (`crate::`)
4. Relative imports (`super::`, `self::`)

Separate groups with blank lines. Alphabetically sort within groups.

Example:
```rust
use std::io;
use std::str::FromStr;

use common::bitboard::Square;
use rayon::prelude::*;

use crate::board::Board;
use crate::chess_move::ChessMove;

use super::helper::Helper;
```

### Pre-Commit Hook

Located at `.git/hooks/pre-commit`. Runs automatically before commits:
- `cargo fmt -- --check` - Verify formatting
- `cargo clippy -- -D warnings` - Enforce zero warnings

If checks fail, commit is rejected. Fix issues before committing.

### Code Quality Enforcement

- **`rustfmt.toml`** - Configuration file documenting the import style standard and other formatting rules.
- **Pre-commit hook** (`.git/hooks/pre-commit`) - Automatically runs `cargo fmt -- --check` and `cargo clippy -- -D warnings` before each commit to ensure code quality and consistency.

## Profiling and Optimization

This section provides a systematic approach to profiling and optimizing codepaths in the chess engine. Use this as a template when optimizing any component.

### Profiling Tools

#### CPU Profiling (Text-Based)

**macOS:**
```bash
chess count-positions --depth 6 2>&1 &
CHESS_PID=$!
sleep 1
sample $CHESS_PID 30 -file /tmp/profile.txt
wait $CHESS_PID
tail -n 100 /tmp/profile.txt  # See "Sort by top of stack" summary
```

**Linux:**
```bash
chess count-positions --depth 6 2>&1 &
CHESS_PID=$!
sleep 1
perf record -p $CHESS_PID sleep 30
perf report
```

Analyze the "Sort by top of stack" section to identify hot functions.

#### Memory Profiler (Built-In)

The engine includes instrumentation for tracking allocations:
- Automatically tracks board clones and MoveGenerator allocations
- Output shown after `count-positions` command
- Add custom counters using `MemoryProfiler::record_*()` methods in `src/diagnostics/memory_profiler.rs`

#### Visual Profiling

**Flamegraph:**
```bash
sudo cargo flamegraph --bench pvp_benchmark
```

![Flamegraph of the `pvp_benchmark` benchmark](./pvp_benchmark.svg)

#### Debug Logging for Lock Contention

The engine includes debug-level logging for monitoring lock contention in critical sections. These messages are useful for identifying synchronization bottlenecks:

**Enable debug logging:**
```bash
RUST_LOG=debug chess play --depth 4
```

**Key debug messages:**
- `Slow killer get lock` - Indicates when acquiring the killer moves lock takes >100µs
- `Slow killer store lock` - Indicates when acquiring the killer moves lock for storage takes >100µs

These messages help identify when thread contention on the `killer_moves` mutex is impacting performance. High frequency of these messages suggests the parallel search strategy may need adjustment or the killer move storage mechanism should be optimized.

**Note:** These messages are logged at debug level by default and won't appear in normal operation. Use `RUST_LOG=debug` to enable them during profiling.

### Optimization Workflow

#### 1. Establish Baseline

Measure performance before making changes:
```bash
chess count-positions --depth 5 | tail -n 1
```

Record key metrics: positions/second, duration, memory profiler stats.

**Quick feedback loops are essential**: Use shallow depths (depth 4-5) during development for fast iteration. Deep profiling (depth 6+) should be reserved for final validation, as it takes significantly longer. The `count-positions` command provides immediate feedback on performance changes, allowing you to iterate quickly and validate optimizations before investing time in comprehensive profiling.

#### 2. Profile to Identify Bottlenecks

Use CPU profiling to find:
- Thread synchronization overhead (e.g., `__psynch_cvwait` on macOS)
- Excessive allocations (board clones, object creations)
- Algorithmic bottlenecks (not just hot functions)

#### 3. Focus on Algorithmic Improvements

Prioritize optimizations that:
- **Reduce operation counts**: Eliminate redundant allocations, avoid unnecessary cloning
- **Improve parallelization strategy**: Multi-depth parallelization, conditional cloning
- **Share resources**: Pass references instead of creating new instances

#### 4. Measure After Each Change

**Critical:** Always verify correctness before measuring performance:
```bash
cargo test  # Must pass before measuring
chess count-positions --depth 5 | tail -n 1
```

Compare results to baseline and verify position counts match exactly.

**Maintain quick feedback cycles**: After each optimization, run tests and a quick performance check. This allows you to:
- Catch regressions immediately
- Validate that optimizations actually improve performance
- Iterate rapidly without waiting for long-running benchmarks
- Build confidence before investing in deeper profiling

Only run comprehensive profiling (depth 6+, full CPU profiling) after you've validated improvements with quick checks.

### Key Lessons

- **High sample counts ≠ slow functions**: Functions called frequently may show high counts but be fast
- **Compiler optimizations**: Modern compilers with LTO already optimize many micro-operations
- **Real bottlenecks**: Often in resource allocation patterns, parallelization strategy, or algorithmic complexity
- **Instrumentation is essential**: Add counters to track allocations, clones, and operation counts
- **System variability**: ±10% variation is normal; measure multiple times for small improvements

### Performance Considerations

- Always test performance changes with release builds (`cargo build --release`)
- Use benchmarks (`cargo bench`) to measure impact of optimizations
- The `pvp_benchmark` simulates engine-vs-engine gameplay for realistic profiling

