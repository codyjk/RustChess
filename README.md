# `chess`
A high-performance chess engine written in Rust, using the classical [alpha-beta pruning](https://en.wikipedia.org/wiki/Alpha–beta_pruning) algorithm for best-move selection. The engine achieves ~117M positions/second in pure search and features UCI protocol support for integration with chess GUIs.

![Example of player playing against the engine](./demo.gif)

## Installation

Clone this repository, and then run:

```shell
cargo install --path .
```

Once installed, you can run the engine with `chess`, so long as you have the `chess` binary in your `PATH` (e.g. `export PATH="$PATH:$HOME/.cargo/bin"`).

## Usage

```console
$ chess --help
chess 1.0.0
A classical chess engine implemented in Rust ♛

USAGE:
    chess <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    benchmark-alpha-beta       Run a quick alpha-beta performance benchmark on a curated set of positions. Reports
                               nodes/sec, transposition table hit rate, and other metrics for fast iteration. Use
                               `--depth` (default: 4) and `--parallel` flag to test different configurations.
    calculate-best-move        Use the chess engine to determine the best move from a given position, provided in
                               FEN notation with `--fen` (required). You can optionally specify the depth of the
                               search with the `--depth` arg (default: 4).
    count-positions            Count the number of possible positions for a given `--depth` (default: 4), and
                               reports the time it took to do so. By default, this searches all possible positions.
                               The routine can be run with alpha-beta pruning by selecting `--strategy alpha-beta`.
    determine-stockfish-elo    Determine the ELO rating of the engine at a given `--depth` (default: 4) and
                               `--starting-elo` (default: 1000). The engine will increment the Stockfish ELO until
                               it plateaus at a 50% win rate, at which point the rating is reported.
    help                       Prints this message or the help of the given subcommand(s)
    play                       Play a game against the computer, which will search for the best move using alpha-
                               beta pruning at the given `--depth` (default: 4). Your starting color will be
                               chosen at random unless you specify with `--color`. The initial position can be
                               specified using FEN notation with `--fen` (default: starting position).
    pvp                        Play a game against another human on this local machine. The initial position can be
                               specified using FEN notation with `--fen` (default: starting position).
    uci                        Start UCI (Universal Chess Interface) mode for integration with external chess GUIs
                               like Arena, cutechess-cli, or lichess. Reads UCI commands from stdin and responds on
                               stdout.
    watch                      Watch the computer play against itself at the given `--depth` (default: 4). The
                               initial position can be specified using FEN notation with `--fen` (default: starting
                               position).

```

### Starting from a custom position

You can start a game from any valid chess position by specifying it in FEN (Forsyth–Edwards Notation) format. For example:

```console
$ chess play --fen "rnbqkbnr/pp1ppppp/8/2p5/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2"
```

This starts a game from the Sicilian Defense position after 1.e4 c5 2.Nf3. The default starting position is used if no FEN is specified.

The `--fen` parameter is available for the `play`, `pvp`, and `watch` commands. Each command will validate the FEN string and ensure it represents a legal chess position before starting the game.


### Calculating the best move from a given position

There is also the option to calculate the best move from a given position. For example:

```console
$ chess calculate-best-move --fen "1Q6/8/8/8/8/k1K5/8/8 w - - 0 1"
Qb3#
```

This evaluates the position using the engine at a default `--depth` of `4`, and writes the result to `stdout` in algebraic notation.

### UCI Protocol Support

The engine supports the Universal Chess Interface (UCI) protocol, allowing it to integrate with external chess GUIs and online platforms:

```console
$ chess uci
```

This starts UCI mode, where the engine reads UCI commands from `stdin` and responds on `stdout`. You can use this with popular chess GUIs like Arena, cutechess-cli, or for integration with online platforms like lichess.

## Performance

### Throughput

On an M1 MacBook Pro, the engine achieves approximately 117 million positions per second in pure depth-first search.

```console
$ chess count-positions --depth 6
depth: 1, positions: 420, positions per second: 568335.5886332883
depth: 2, positions: 9322, positions per second: 17655303.030303027
depth: 3, positions: 206603, positions per second: 39693179.63496637
depth: 4, positions: 5072212, positions per second: 58039110.68392205
depth: 5, positions: 124132536, positions per second: 110713308.06889361
depth: 6, positions: 3320034396, positions per second: 117432945.10356736
total positions: 3449455489, total duration: 29.486818s, positions per second: 116982968.08424701

Board clones: 5286510
MoveGen creates: 1
```

This is a pure depth-first search of all possible positions - no pruning is applied.

[Alpha-beta pruning](https://en.wikipedia.org/wiki/Alpha–beta_pruning), which incorporates the engine's scoring heuristic to prune branches of the search tree, is used to search for the "best" move in actual gameplay. Using this approach, the engine achieves approximately 488,000 positions per second:

```console
$ chess count-positions --depth 6 --strategy alpha-beta
depth: 1, positions: 20, positions per second: 6942.034015966678
depth: 2, positions: 420, positions per second: 153677.27771679472
depth: 3, positions: 1684, positions per second: 240331.0974739546
depth: 4, positions: 12692, positions per second: 636381.8692338548
depth: 5, positions: 52373, positions per second: 306079.75033458206
depth: 6, positions: 358498, positions per second: 536615.813864374
total positions: 425687, total duration: 871.746ms, positions per second: 488315.40379881294

Board clones: 120
MoveGen creates: 13
```

Note that with alpha-beta, the number of positions searched is dramatically reduced (425K vs 3.4B) due to effective pruning, and the total search completes in under a second. This demonstrates that the goal of alpha-beta isn't raw throughput, but rather finding the best move quickly by eliminating irrelevant branches. The low latency enables the engine to reach much higher search depths during actual gameplay.

For more realistic gameplay performance on curated positions, you can use the `benchmark-alpha-beta` subcommand:

```console
$ chess benchmark-alpha-beta --depth 6
======================================================================
Alpha-Beta Performance Benchmark (depth: 6, parallel: false)
======================================================================
...
======================================================================
SUMMARY
----------------------------------------------------------------------
  Total nodes:       6,030,428
  Total time:            46.56s
  Avg speed:               130k nodes/s
  TT hit rate:             4.3%
======================================================================
```

These figures vary by hardware. To achieve the best performance, make sure to use the release build, which leverages [compiler optimizations](./Cargo.toml#L28-L33):

### Gameplay

To measure the engine's performance in actual gameplay, use the `determine-stockfish-elo` subcommand. This will increment the Stockfish ELO until it plateaus at a 50% win rate, at which point the rating is reported.

```sh
chess determine-stockfish-elo --depth 6 --starting-elo 2000
```

At alpha-beta search depth 6, you can observe the engine winning against Stockfish playing at a 2000 ELO.

## Implementation details

There are numerous optimizations used to increase the engine's performance. This list is not exhaustive, but should give you a sense of the techniques used:
* The board state is represented using [bitboards](common/src/bitboard/bitboard.rs) (64-bit integers) and [squares](common/src/bitboard/square.rs) (newtype-wrapped u8 indices). This enables the engine to leverage the CPU's bitwise operations to quickly calculate moves, attacks, and other common board state changes.
* The [alpha-beta search algorithm](src/alpha_beta_searcher/mod.rs) is implemented as a generic, game-agnostic algorithm using Rust traits. This allows for clean separation of concerns and comprehensive testing of the search algorithm independent of chess-specific logic.
* [Alpha-beta pruning](https://en.wikipedia.org/wiki/Alpha–beta_pruning) is used to quickly eliminate branches of the search tree that are unlikely to lead to a winning position. The move order is sorted in an attempt to prioritize the "best" moves first, so that worse moves come later in the search and are therefore pruned (and not searched entirely), reducing the search space/time.
* **Move ordering optimizations** including MVV-LVA (Most Valuable Victim - Least Valuable Attacker) for capture ordering, killer move heuristics, and principal variation ordering significantly improve search efficiency.
* **Parallel search** is enabled by default for root-level move exploration, leveraging multi-core processors for improved performance.
* **Transposition tables** cache previously evaluated positions to avoid redundant computation during the search.
* The [Zobrist hashing](./precompile/src/zobrist/mod.rs) tables are generated at compile time using the [precompile](./precompile/src/main.rs) build script. This hashing approach enables quick incremental hashing of the board state so that various computations can be cached (e.g. move generation) by the engine during gameplay.
* **UCI protocol support** enables the engine to integrate with external chess GUIs and online platforms like lichess.
* **Modern TUI** built with ratatui provides an enhanced interactive experience with real-time game visualization.
* Macros are used throughout the codebase to improve the developer experience. See below for one example.

```rust
use crate::board::castle_rights::CastleRights;

// The `chess_position!` macro is used to instantiate a board state from
// an ascii representation of the board. For example, here is the starting
// position:

let board = chess_position! {
    rnbqkbnr
    pppppppp
    ........
    ........
    ........
    ........
    PPPPPPPP
    RNBQKBNR
}

// This is used extensively in tests, where various positions are instantiated
// to exercise the engine's logic with. For example:

#[test]
fn test_find_back_rank_mate_in_2_black() {
    let mut context = SearchContext::new(4);

    let mut board = chess_position! {
        ....r..k
        ....q...
        ........
        ........
        ........
        ........
        .....PPP
        R.....K.
    };
    board.set_turn(Color::Black);
    board.lose_castle_rights(CastleRights::all());

    let best_move = search_best_move(&mut context, &mut board).unwrap();
    // ... assertions ...
}
```

## Profiling

For detailed profiling and optimization guidance, see [PERFORMANCE_GUIDE.md](./PERFORMANCE_GUIDE.md).

### Quick Start

**Visual profiling with flamegraph:**
```shell
sudo cargo flamegraph --bench pvp_benchmark
```

![Flamegraph of the `pvp_benchmark` benchmark](./pvp_benchmark.svg)

**CPU profiling:**
- **macOS**: `sample $PID 30 -file /tmp/profile.txt`
- **Linux**: `perf record -p $PID sleep 30 && perf report`

**Memory profiling:**
The engine includes built-in instrumentation that automatically tracks allocations. Run any command and check the output for memory profiler stats.

Various other [benchmarks](https://doc.rust-lang.org/cargo/commands/cargo-bench.html) are available in the [`benches`](./benches) directory.

## Codebase structure

* [`common`](./common) contains code that is shared between the engine and the precompiler. This includes the [`Bitboard`](./common/src/bitboard/bitboard.rs) type (64-bit integer for sets of squares) and the [`Square`](./common/src/bitboard/square.rs) type (newtype-wrapped u8 for individual squares).
* [`precompile`](./precompile) contains the precompiler, which generates the [`ZobristHashTable`](./precompile/src/zobrist/mod.rs) tables and [magic bitboard](./precompile/src/magic/find_magics.rs) calculation (see [this](https://www.chessprogramming.org/Magic_Bitboards) for background).
* [`src`](./src) contains the engine's main logic:
  * [`prelude`](./src/prelude.rs) - Common types re-exported for convenience (`Board`, `Color`, `Piece`, `ChessMove`, `Bitboard`, `Square`)
  * [`alpha_beta_searcher`](./src/alpha_beta_searcher/mod.rs) - Generic alpha-beta search algorithm, independent of chess
  * [`chess_search`](./src/chess_search/mod.rs) - Chess-specific trait implementations for the search algorithm
  * [`board`](./src/board/mod.rs) - Chess board state representation, including newtype wrappers (`CastleRights`, `HalfmoveClock`, `FullmoveNumber`) and state management (`StateStack`)
  * [`chess_move`](./src/chess_move/mod.rs) - Chess move types and application logic
  * [`move_generator`](./src/move_generator/mod.rs) - Chess move generation
  * [`game`](./src/game/mod.rs) - Game loop and engine coordination, with separate `InputSource` and `GameRenderer` traits for modularity

## Module structure standards

The codebase follows idiomatic Rust module organization practices:

### Module organization

* **`mod.rs` files** serve only as module declarations and re-exports. They do not contain substantial implementation code.
* **Implementation files** are placed in dedicated `.rs` files within their module directories (e.g., `board/board.rs`, `evaluate/evaluation.rs`).
* **Module-level docstrings** (`//!`) are required at the top of all `mod.rs` files and implementation files to describe the module's purpose.

### Import organization

All imports follow a consistent ordering standard (enforced by the pre-commit hook):

1. **Standard library** (`std::`, `core::`)
2. **External crates** (third-party dependencies like `common`, `rayon`, `smallvec`, etc.)
3. **Crate imports** (`crate::`)
4. **Relative imports** (`super::`, `self::`)

Each group is separated by a blank line. Within each group, imports are alphabetically sorted when possible and grouped by module when importing multiple items from the same module.

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

### Code quality enforcement

* **`rustfmt.toml`** - Configuration file documenting the import style standard and other formatting rules.
* **Pre-commit hook** (`.git/hooks/pre-commit`) - Automatically runs `cargo fmt -- --check` and `cargo clippy -- -D warnings` before each commit to ensure code quality and consistency.
