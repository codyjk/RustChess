use std::time::Duration;

use chess::board::color::Color;
use chess::game::engine::EngineConfig;
use chess::game::mode::{ComputerVsComputer, HumanVsComputer, HumanVsHuman};
use chess::game::position_counter::{run_count_positions, CountPositionsStrategy};
use chess::game::r#loop::GameLoop;
use chess::game::stockfish_elo::determine_stockfish_elo;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "chess",
    about = "A classical chess engine implemented in Rust ♛"
)]
enum Chess {
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
    #[structopt(
        name = "play",
        about = "Play a game against the computer, which will search for the best move using alpha-beta pruning at the given `--depth` (default: 4). Your starting color will be chosen at random unless you specify with `--color`."
    )]
    Play {
        #[structopt(short, long, default_value = "4")]
        depth: u8,
        #[structopt(short = "c", long = "color", default_value = "random")]
        color: Color,
    },
    #[structopt(
        name = "pvp",
        about = "Play a game against another human on this local machine."
    )]
    Pvp,
    #[structopt(
        name = "watch",
        about = "Watch the computer play against itself at the given `--depth` (default: 4)."
    )]
    Watch {
        #[structopt(short, long, default_value = "4")]
        depth: u8,
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
}

fn main() {
    env_logger::init();

    let args = Chess::from_args();

    match args {
        Chess::Play { depth, color } => {
            let mode = HumanVsComputer { human_color: color };
            let config = EngineConfig {
                search_depth: depth,
            };
            let mut game = GameLoop::new(mode, config);
            game.run();
        }
        Chess::Watch { depth } => {
            let mode = ComputerVsComputer {
                delay_between_moves: Some(Duration::from_millis(1000)),
            };
            let config = EngineConfig {
                search_depth: depth,
            };
            let mut game = GameLoop::new(mode, config);
            game.run();
        }
        Chess::Pvp => {
            let mode = HumanVsHuman;
            let config = EngineConfig { search_depth: 0 };
            let mut game = GameLoop::new(mode, config);
            game.run();
        }
        Chess::CountPositions { depth, strategy } => run_count_positions(depth, strategy),
        Chess::DetermineStockfishElo {
            depth,
            starting_elo,
        } => determine_stockfish_elo(depth, starting_elo),
    }
}
