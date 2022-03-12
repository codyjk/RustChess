use chess::board::color::Color;
use chess::board::magic::run_find_magic;
use chess::board::Board;
use chess::game::modes::{computer_vs_computer, play_computer, player_vs_player};
use chess::moves::count_positions;
use chess::moves::targets::Targets;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "chess", about = "chess engine cli")]
enum Chess {
    CountPositions {
        #[structopt(short, long, default_value = "3")]
        depth: u8,
    },
    Play {
        #[structopt(short, long, default_value = "3")]
        depth: u8,
    },
    Pvp,
    Watch {
        #[structopt(short, long, default_value = "3")]
        depth: u8,
    },
    FindMagic,
}

fn main() {
    let args = Chess::from_args();
    match args {
        Chess::CountPositions { depth } => run_count_positions(depth),
        Chess::Play { depth } => play_computer(depth),
        Chess::Watch { depth } => computer_vs_computer(0, 1000, depth),
        Chess::Pvp => player_vs_player(),
        Chess::FindMagic => run_find_magic(),
    }
}

fn run_count_positions(depth: u8) {
    let depths = 0..=depth;
    let mut targets = Targets::new();

    for depth in depths {
        let mut board = Board::starting_position();
        let count = count_positions(depth, &mut board, &mut targets, Color::White);

        println!("depth: {}, positions: {}", depth, count);
    }
}
