# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
make install          # Build with native CPU opts + install (default target)
make build            # Release build only
make dev              # Fast dev build (unoptimized)
make test             # Run all tests (cargo test)
make bench            # Run benchmarks (cargo bench)
make lint             # cargo fmt --check + cargo clippy -- -D warnings
make pre-commit       # lint + test
```

Run a single test: `cargo test alpha_beta_searcher::tests::test_alpha_beta_finds_winning_move`
Run a test module: `cargo test board::tests::`
Run with output: `cargo test -- --nocapture`

Build with instrumentation: `cargo build --release --features instrumentation`

## Architecture

Three-layer design separating generic search from chess-specific logic:

1. **`alpha_beta_searcher/`** — Game-agnostic alpha-beta search with iterative deepening, quiescence search, transposition table (DashMap-backed), and killer moves (thread-local). Defines traits in `traits.rs`: `GameState`, `GameMove`, `MoveGenerator`, `Evaluator`, `MoveOrderer`. Tests use a Nim game implementation to validate the algorithm independently of chess.

2. **`chess_search/`** — Implements the generic search traits for chess. `implementation.rs` bridges the search algorithm to chess types. `move_orderer.rs` handles PV moves, MVV-LVA capture ordering, killer moves, and history heuristic.

3. **`board/` + `move_generator/`** — Board state with bitboard representation, `StateStack` for undo (apply/undo pattern avoids cloning during search), and magic bitboard move generation. `chess_move/` has an enum with variants: Standard, Castle, EnPassant, PawnPromotion, each implementing `ChessMoveEffect`.

### Other key modules
- **`precompile/`** — Build script (`build = "precompile/src/main.rs"`) generates Zobrist hash tables, magic bitboard lookup tables, and opening book at compile time
- **`common/`** — `Bitboard` (u64 wrapper) and `Square` (u8 newtype) shared between engine and precompiler
- **`game/`** — Game loop with `InputSource`/`GameRenderer` trait abstractions for I/O modularity
- **`uci/`** — UCI protocol state machine for GUI integration
- **`tui/`** — ratatui-based terminal UI, colors customizable via `tui_colors.toml`
- **`evaluate/`** — Material scoring + piece-square tables
- **`cli/`** — StructOpt subcommands (play, watch, pvp, uci, calculate-best-move, count-positions, benchmark-alpha-beta, determine-stockfish-elo)

### Key patterns
- Parallel search at root level only, using Rayon. Conditional cloning (only when parallelizing with 10+ moves).
- `SmallVec<[ChessMove; 32]>` for move lists to avoid heap allocation in typical positions.
- `chess_position!` macro for creating board state from ASCII art in tests.
- `prelude.rs` re-exports common types: `Board`, `Color`, `Piece`, `ChessMove`, `Bitboard`, `Square`.

## Code Standards

- **Import order** (enforced by pre-commit hook): std → external crates → `crate::` → `super::`/`self::`, separated by blank lines, alphabetically sorted within groups
- **Module structure**: `mod.rs` files contain only declarations/re-exports, no implementation
- **Module docstrings**: `//!` comment required at top of all module and implementation files
- **Max line width**: 100 characters
- **Edition**: 2018
- **Pre-commit hook** runs `cargo fmt -- --check` and `cargo clippy -- -D warnings`

## Performance Testing

Always use release builds for performance measurement. Quick feedback loop:
```bash
cargo test && chess count-positions --depth 5 | tail -n 1
```
Use depth 4-5 for iteration; depth 6+ for final validation only. Position counts must match exactly after optimization changes (correctness check).
