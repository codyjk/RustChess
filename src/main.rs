use chess::board::color::Color;
use chess::game::computer_vs_computer::computer_vs_computer;
use chess::game::human_vs_computer::play_computer;
use chess::game::player_vs_player::player_vs_player;
use chess::game::position_counter::{run_count_positions, CountPositionsStrategy};
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
        Chess::CountPositions { depth, strategy } => run_count_positions(depth, strategy),
        Chess::Play { depth, color } => play_computer(depth, color),
        Chess::Watch { depth } => computer_vs_computer(0, 1000, depth),
        Chess::Pvp => player_vs_player(),
        Chess::DetermineStockfishElo {
            depth,
            starting_elo,
        } => determine_stockfish_elo(depth, starting_elo),
    }
}
