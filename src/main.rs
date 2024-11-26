use std::time::Duration;

use chess::board::color::Color;
use chess::board::Board;
use chess::game::engine::{Engine, EngineConfig};
use chess::game::mode::{ComputerVsComputer, HumanVsComputer, HumanVsHuman};
use chess::game::position_counter::{run_count_positions, CountPositionsStrategy};
use chess::game::r#loop::GameLoop;
use chess::game::stockfish_elo::determine_stockfish_elo;
use chess::input_handler::fen::STARTING_POSITION_FEN;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
    name = "chess",
    about = "A classical chess engine implemented in Rust ♛"
)]
enum Chess {
    // Gameplay commands
    #[structopt(
        name = "play",
        about = "Play a game against the computer, which will search for the best move using alpha-beta pruning at the given `--depth` (default: 4). Your starting color will be chosen at random unless you specify with `--color`. The initial position can be specified using FEN notation with `--fen` (default: starting position)."
    )]
    Play {
        #[structopt(short, long, default_value = "4")]
        depth: u8,
        #[structopt(short = "c", long = "color", default_value = "random")]
        color: Color,
        #[structopt(long = "fen", default_value = STARTING_POSITION_FEN)]
        starting_position: Board,
    },
    #[structopt(
        name = "pvp",
        about = "Play a game against another human on this local machine. The initial position can be specified using FEN notation with `--fen` (default: starting position)."
    )]
    Pvp {
        #[structopt(long = "fen", default_value = STARTING_POSITION_FEN)]
        starting_position: Board,
    },
    #[structopt(
        name = "watch",
        about = "Watch the computer play against itself at the given `--depth` (default: 4). The initial position can be specified using FEN notation with `--fen` (default: starting position)."
    )]
    Watch {
        #[structopt(short, long, default_value = "4")]
        depth: u8,
        #[structopt(long = "fen", default_value = STARTING_POSITION_FEN)]
        starting_position: Board,
    },

    // Utility commands
    #[structopt(
        name = "calculate-best-move",
        about = "Use the chess engine to determine the best move from a given position, provided in FEN notation with `--fen` (required). You can optionally specify the depth of the search with the `--depth` arg (default: 4)."
    )]
    CalculateBestMove {
        #[structopt(short, long, default_value = "4")]
        depth: u8,
        #[structopt(long = "fen")]
        starting_position: Board,
    },

    #[structopt(
        name = "determine-stockfish-elo",
        about = "Determine the ELO rating of the engine at a given `--depth` (default: 4) and `--starting-elo` (default: 1000). The engine will increment the Stockfish ELO until it plateaus at a 50% win rate, at which point the rating is reported."
    )]
    DetermineStockfishElo {
        #[structopt(short, long, default_value = "4")]
        depth: u8,
        #[structopt(short, long, default_value = "1000")]
        starting_elo: u32,
    },
    #[structopt(
        name = "count-positions",
        about = "Count the number of possible positions for a given `--depth` (default: 4), and reports the time it took to do so. By default, this searches all possible positions. The routine can be run with alpha-beta pruning by selecting `--strategy alpha-beta`."
    )]
    CountPositions {
        #[structopt(short, long, default_value = "4")]
        depth: u8,
        #[structopt(short, long, default_value = "all")]
        strategy: CountPositionsStrategy,
    },
}

fn main() {
    env_logger::init();

    let args = Chess::from_args();

    match args {
        Chess::Play {
            depth,
            color,
            starting_position,
        } => {
            let mode = HumanVsComputer { human_color: color };
            let config = EngineConfig {
                search_depth: depth,
                starting_position,
            };
            let mut game = GameLoop::new(mode, config);
            game.run();
        }
        Chess::Watch {
            depth,
            starting_position,
        } => {
            let mode = ComputerVsComputer {
                delay_between_moves: Some(Duration::from_millis(1000)),
            };
            let config = EngineConfig {
                search_depth: depth,
                starting_position,
            };
            let mut game = GameLoop::new(mode, config);
            game.run();
        }
        Chess::Pvp { starting_position } => {
            let mode = HumanVsHuman;
            let config = EngineConfig {
                search_depth: 0,
                starting_position,
            };
            let mut game = GameLoop::new(mode, config);
            game.run();
        }
        Chess::CalculateBestMove {
            depth,
            starting_position,
        } => {
            let config = EngineConfig {
                search_depth: depth,
                starting_position,
            };
            let mut engine = Engine::with_config(config);
            let valid_moves = engine.get_valid_moves();
            if valid_moves.len() == 0 {
                eprintln!("There are no valid moves in the given position.");
                return;
            }
            match engine.get_best_move() {
                Ok(best_move) => {
                    let algebraic_move = valid_moves
                        .iter()
                        .find(|(chess_move, _)| chess_move == &best_move)
                        .map(|(_, algebraic_notation)| algebraic_notation.as_str())
                        .unwrap();
                    println!("{}", algebraic_move);
                }
                Err(err) => {
                    eprintln!("Failed to calculate best move: {}", err);
                }
            }
        }
        Chess::DetermineStockfishElo {
            depth,
            starting_elo,
        } => determine_stockfish_elo(depth, starting_elo),
        Chess::CountPositions { depth, strategy } => run_count_positions(depth, strategy),
    }
}
