//! CLI argument parsing using StructOpt.

use structopt::StructOpt;

use crate::cli::commands::{
    calculate_best_move::CalculateBestMoveArgs, count_positions::CountPositionsArgs,
    determine_stockfish_elo::DetermineStockfishEloArgs, play::PlayArgs, pvp::PvpArgs, uci::UciArgs,
    watch::WatchArgs,
};

#[derive(StructOpt)]
#[structopt(
    name = "chess",
    about = "A classical chess engine implemented in Rust ♛"
)]
pub enum Chess {
    #[structopt(
        name = "play",
        about = "Play a game against the computer, which will search for the best move using alpha-beta pruning at the given `--depth` (default: 4). Your starting color will be chosen at random unless you specify with `--color`. The initial position can be specified using FEN notation with `--fen` (default: starting position)."
    )]
    Play(PlayArgs),
    #[structopt(
        name = "pvp",
        about = "Play a game against another human on this local machine. The initial position can be specified using FEN notation with `--fen` (default: starting position)."
    )]
    Pvp(PvpArgs),
    #[structopt(
        name = "watch",
        about = "Watch the computer play against itself at the given `--depth` (default: 4). The initial position can be specified using FEN notation with `--fen` (default: starting position)."
    )]
    Watch(WatchArgs),
    #[structopt(
        name = "calculate-best-move",
        about = "Use the chess engine to determine the best move from a given position, provided in FEN notation with `--fen` (required). You can optionally specify the depth of the search with the `--depth` arg (default: 4)."
    )]
    CalculateBestMove(CalculateBestMoveArgs),
    #[structopt(
        name = "determine-stockfish-elo",
        about = "Determine the ELO rating of the engine at a given `--depth` (default: 4) and `--starting-elo` (default: 1000). The engine will increment the Stockfish ELO until it plateaus at a 50% win rate, at which point the rating is reported."
    )]
    DetermineStockfishElo(DetermineStockfishEloArgs),
    #[structopt(
        name = "count-positions",
        about = "Count the number of possible positions for a given `--depth` (default: 4), and reports the time it took to do so. By default, this searches all possible positions. The routine can be run with alpha-beta pruning by selecting `--strategy alpha-beta`."
    )]
    CountPositions(CountPositionsArgs),
    #[structopt(
        name = "uci",
        about = "Start UCI (Universal Chess Interface) mode for integration with external chess GUIs like Arena, cutechess-cli, or lichess. Reads UCI commands from stdin and responds on stdout."
    )]
    Uci(UciArgs),
}

impl crate::cli::commands::Command for Chess {
    fn execute(self) {
        macro_rules! execute_command {
            ($($variant:ident($cmd:ident)),+ $(,)?) => {
                match self {
                    $(Self::$variant($cmd) => $cmd.execute(),)+
                }
            };
        }

        execute_command! {
            Play(cmd),
            Pvp(cmd),
            Watch(cmd),
            CalculateBestMove(cmd),
            DetermineStockfishElo(cmd),
            CountPositions(cmd),
            Uci(cmd),
        }
    }
}
