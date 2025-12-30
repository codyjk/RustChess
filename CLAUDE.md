# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A high-performance chess engine written in Rust using classical alpha-beta pruning for move selection. The engine achieves ~45M positions/second in pure search and ~900K positions/second with alpha-beta pruning on M1 hardware.

## Essential Commands

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

**`src/`** - Main engine implementation
- `prelude.rs` - Common type re-exports (`Board`, `Color`, `Piece`, `ChessMove`, `Bitboard`, `Square`)
- `alpha_beta_searcher/` - Generic search algorithm with traits
  - `search.rs` - Core alpha-beta implementation with transposition tables
  - `traits.rs` - Game-agnostic trait definitions
  - `transposition_table.rs` - Hash table for position caching
- `chess_search/` - Chess-specific search implementations
  - `implementation.rs` - Implements `MoveGenerator`, `Evaluator` traits for chess
  - `move_orderer.rs` - Move ordering heuristics for pruning optimization
- `board/` - Board state representation
  - `board.rs` - Main `Board` struct with piece placement and game state
  - `piece_set.rs` - Per-color piece bitboards
  - `state_stack.rs` - Undo stack for move reversal
  - `castle_rights.rs`, `halfmove_clock.rs`, `fullmove_number.rs` - Game state newtypes
- `chess_move/` - Move types and application
  - `chess_move.rs` - `ChessMove` enum with all move variants
  - `chess_move_effect.rs` - Trait for applying moves to board
  - `standard.rs`, `castle.rs`, `en_passant.rs`, `pawn_promotion.rs` - Move implementations
  - `algebraic_notation.rs` - Display moves in standard notation
- `move_generator/` - Legal move generation
  - `generator.rs` - Main move generation logic with caching
  - `targets.rs` - Calculate attack/movement targets for pieces
  - `magic_table.rs` - Magic bitboards for sliding pieces
- `evaluate/` - Position scoring
  - `evaluation.rs` - Board evaluation heuristic (material + position)
  - `evaluation_tables.rs` - Piece-square tables for positional scoring
- `game/` - Game loop and I/O
  - `loop.rs` - Main game loop using `InputSource` and `GameRenderer` traits
  - `input_source.rs`, `renderer.rs` - Trait abstractions for I/O
  - `engine.rs` - Computer player using alpha-beta search
  - `stockfish_interface.rs` - UCI protocol for testing against Stockfish
- `input_handler/` - FEN parsing and user input
- `cli/` - Command-line interface using `structopt`

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
- **`mod.rs` files**: Only module declarations and re-exports, no implementation
- **Implementation files**: Dedicated `.rs` files (e.g., `board/board.rs`, not in `mod.rs`)
- **Module docstrings**: Required `//!` comment at top of all module and implementation files

### Import Organization
Enforced by pre-commit hook and documented in `rustfmt.toml`:

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

### Performance Considerations
- Always test performance changes with release builds (`cargo build --release`)
- Use benchmarks (`cargo bench`) to measure impact of optimizations
- Profile with `cargo flamegraph --bench pvp_benchmark` to identify bottlenecks
- The `pvp_benchmark` simulates engine-vs-engine gameplay for realistic profiling
