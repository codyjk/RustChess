use chess::board::color::Color;
use chess::game::modes::{
    computer_vs_computer, play_computer, player_vs_player, run_count_positions,
    CountPositionsStrategy,
};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "chess", about = "chess engine cli")]
enum Chess {
    CountPositions {
        #[structopt(short, long, default_value = "4")]
        depth: u8,
        #[structopt(short, long, default_value = "all")]
        strategy: CountPositionsStrategy,
    },
    Play {
        #[structopt(short, long, default_value = "4")]
        depth: u8,
        #[structopt(short = "c", long = "color", default_value = "random")]
        color: Color,
    },
    Pvp,
    Watch {
        #[structopt(short, long, default_value = "4")]
        depth: u8,
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
    }
}
