# `chess`
A high-performance chess engine written in Rust, using the classical [alpha-beta pruning](https://en.wikipedia.org/wiki/Alpha–beta_pruning) algorithm for best-move selection. The engine achieves ~264M positions/second in pure search and features UCI protocol support for integration with chess GUIs.

![Example of player playing against the engine](./chess.gif)

## Installation

Clone this repository, and then run:

```shell
make
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

### Customizing TUI Colors

The TUI color scheme can be customized by creating a `tui_colors.toml` file in the current working directory. Edit this file to change colors without rebuilding:

```toml
# Light squares (traditional wheat/beige)
light_square = 240, 217, 181

# Dark squares (traditional sienna/brown)
dark_square = 181, 136, 99

# White pieces (traditional white)
piece_white = 255, 255, 255

# Black pieces (traditional dark gray/black)
piece_black = 50, 50, 50
```

Colors are specified as RGB values (0-255). If the file is missing or invalid, default colors are used. Changes take effect immediately on the next run - no rebuild required.

## Performance

### Throughput

On an M1 MacBook Pro, the engine achieves approximately 264 million positions per second in pure depth-first search.

```console
$ chess count-positions --depth 6
depth: 1, positions: 420, positions per second: 665610.1426307448
depth: 2, positions: 9322, positions per second: 42958525.34562212
depth: 3, positions: 206579, positions per second: 81234368.85568225
depth: 4, positions: 5070585, positions per second: 192060338.62353697
depth: 5, positions: 124064942, positions per second: 349045107.3455229
depth: 6, positions: 3317378318, positions per second: 262101541.1108804
total positions: 3446730166, total duration: 13.042077s, positions per second: 264277704.08041602

Board clones: 5285741
MoveGen creates: 1
```

This is a pure depth-first search of all possible positions - no pruning is applied.

[Alpha-beta pruning](https://en.wikipedia.org/wiki/Alpha–beta_pruning), which incorporates the engine's scoring heuristic to prune branches of the search tree, is used to search for the "best" move in actual gameplay. The engine reaches **depth 10 in under 1 second** from the starting position thanks to aggressive pruning and search optimizations.

For gameplay performance on curated positions, use the `benchmark-alpha-beta` subcommand:

```console
$ chess benchmark-alpha-beta --depth 6
======================================================================
Alpha-Beta Performance Benchmark (depth: 6, parallel: false)
======================================================================
...
======================================================================
SUMMARY
----------------------------------------------------------------------
  Total nodes:         514,065
  Total time:             0.50s
  Avg speed:              1025k nodes/s
======================================================================
```

At depth 10 from the starting position, the engine searches ~690K nodes in 0.57s. These figures vary by hardware. To achieve the best performance, make sure to use the release build, which leverages [compiler optimizations](./Cargo.toml#L28-L33):

### Gameplay

To measure the engine's performance in actual gameplay, use the `determine-stockfish-elo` subcommand. This will increment the Stockfish ELO until it plateaus at a 50% win rate, at which point the rating is reported.

```sh
chess determine-stockfish-elo --depth 6 --starting-elo 2000
```

At alpha-beta search depth 6, you can observe the engine winning against Stockfish playing at a 2000 ELO.

## Implementation details

The engine employs a sophisticated combination of algorithms and optimizations to achieve high performance:
* **[Bitboard representation](common/src/bitboard/bitboard.rs)** with [magic bitboards](src/move_generator/magic_table.rs) for sliding pieces (rooks, bishops, queens) enables O(1) attack generation via precomputed lookup tables. The board state uses 64-bit integers for efficient bitwise operations and newtype-wrapped u8 indices for type-safe square indexing.
* **[Alpha-beta search](src/alpha_beta_searcher/search.rs)** with iterative deepening, aspiration windows, and quiescence search. Iterative deepening searches at increasing depths (1..target), using transposition table results to improve move ordering at each level. Aspiration windows narrow the search window around the previous depth's score to reduce nodes. Quiescence search extends beyond the nominal depth for tactical moves to avoid the horizon effect.
* **Aggressive pruning** reduces the search tree dramatically: null move pruning (skip a turn to detect positions too good to need searching), reverse futility pruning (prune entire nodes at shallow depths when the static eval is far above the bound), futility pruning (skip individual quiet moves that cannot reach the bound), and late move reductions with logarithmic scaling (search later moves at reduced depth).
* **Check extensions** extend search depth by 1 ply when in check, preventing the horizon effect from hiding tactical sequences.
* **[Transposition tables](src/alpha_beta_searcher/transposition_table.rs)** use a concurrent hash map (DashMap, 64MB default) with depth-preferred replacement to cache position evaluations by [Zobrist hash](./precompile/src/zobrist/mod.rs), avoiding redundant computation of transposed positions. Each entry stores score, depth, bound type (exact/upper/lower), and the best move for move ordering. Deeper entries are preserved over shallow ones for better hit quality.
* **Advanced move ordering** prioritizes moves likely to cause cutoffs: principal variation moves from the transposition table, [killer moves](src/alpha_beta_searcher/killer_moves.rs) stored in thread-local storage (eliminating lock contention), MVV-LVA (Most Valuable Victim - Least Valuable Attacker) for capture ordering, and history heuristic for quiet moves. Interior nodes use incremental selection (pick-best) instead of a full sort, avoiding O(n log n) sorting of moves never searched due to beta cutoffs.
* **Parallel search** with thread-local killer move storage enables lock-free parallelization at the root level. [Move generation](src/move_generator/generator.rs) uses conditional cloning (only when parallelizing) and MoveGenerator sharing to minimize allocations.
* **[Zobrist hashing](./precompile/src/zobrist/mod.rs)** tables are generated at compile time via the [precompile](./precompile/src/main.rs) build script, enabling incremental position hashing for efficient caching of move generation and transposition table lookups.
* **Generic trait-based architecture** implements the alpha-beta algorithm as a game-agnostic search using Rust traits, enabling clean separation between search logic and chess-specific implementations for comprehensive testing and maintainability.
* **[Simple TUI](src/tui/app.rs)** built with ratatui and crossterm provides real-time game visualization with customizable colors. [UCI protocol support](src/uci/mod.rs) enables integration with external chess GUIs and online platforms like lichess.

## Codebase structure

```
RustChess/
├── common/              # Shared code between engine and precompiler
│   └── src/bitboard/      # Bitboard and Square types
├── precompile/            # Build-time code generation
│   ├── src/zobrist/      # Zobrist hash table generation
│   ├── src/magic/        # Magic bitboard calculation
│   └── src/book/         # Opening book generation
└── src/                   # Main engine implementation
    ├── prelude.rs         # Common type re-exports
    ├── alpha_beta_searcher/  # Generic search algorithm
    ├── chess_search/      # Chess-specific search implementations
    ├── board/            # Board state representation
    ├── chess_move/       # Move types and application
    ├── move_generator/   # Legal move generation
    ├── evaluate/         # Position evaluation
    ├── game/             # Game loop and engine coordination
    ├── book/             # Opening book lookup
    ├── input_handler/    # FEN parsing and input handling
    ├── cli/              # Command-line interface
    ├── uci/              # UCI protocol implementation
    ├── tui/              # Terminal user interface
    └── diagnostics/      # Memory profiling and diagnostics
```

**Key directories:**

* [`common`](./common) - Shared types between engine and precompiler: [`Bitboard`](./common/src/bitboard/bitboard.rs) (64-bit integer for sets of squares) and [`Square`](./common/src/bitboard/square.rs) (newtype-wrapped u8 for individual squares).

* [`precompile`](./precompile) - Build-time code generation: [`ZobristHashTable`](./precompile/src/zobrist/mod.rs) tables and [magic bitboard](./precompile/src/magic/find_magics.rs) calculation (see [this](https://www.chessprogramming.org/Magic_Bitboards) for background).

* [`src`](./src) - Main engine implementation:
  * [`prelude`](./src/prelude.rs) - Common types re-exported for convenience (`Board`, `Color`, `Piece`, `ChessMove`, `Bitboard`, `Square`)
  * [`alpha_beta_searcher`](./src/alpha_beta_searcher/mod.rs) - Generic alpha-beta search algorithm, independent of chess
  * [`chess_search`](./src/chess_search/mod.rs) - Chess-specific trait implementations for the search algorithm
  * [`board`](./src/board/mod.rs) - Chess board state representation, including newtype wrappers (`CastleRights`, `HalfmoveClock`, `FullmoveNumber`) and state management (`StateStack`)
  * [`chess_move`](./src/chess_move/mod.rs) - Chess move types and application logic
  * [`move_generator`](./src/move_generator/mod.rs) - Chess move generation with magic bitboards
  * [`evaluate`](./src/evaluate/mod.rs) - Position evaluation (material + piece-square tables)
  * [`game`](./src/game/mod.rs) - Game loop and engine coordination, with separate `InputSource` and `GameRenderer` traits for modularity
  * [`book`](./src/book/mod.rs) - Opening book lookup for move suggestions
  * [`input_handler`](./src/input_handler/mod.rs) - FEN parsing and position validation
  * [`cli`](./src/cli/mod.rs) - Command-line interface with subcommands
  * [`uci`](./src/uci/mod.rs) - UCI protocol implementation for GUI integration
  * [`tui`](./src/tui/mod.rs) - Terminal user interface with ratatui
  * [`diagnostics`](./src/diagnostics/mod.rs) - Memory profiling and performance diagnostics

## Contributing

For information on development setup, architecture, code standards, and profiling/optimization workflows, see [CONTRIBUTING.md](./CONTRIBUTING.md).
