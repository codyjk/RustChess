use chess::board::color::Color;
use chess::board::magic::run_find_magic;
use chess::board::Board;
use chess::game::modes::{computer_vs_computer, play_computer, player_vs_player};
use chess::moves::count_positions;
use chess::moves::ray_table::RayTable;
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
    Watch,
    FindMagic,
}

fn main() {
    let args = Chess::from_args();
    match args {
        Chess::CountPositions { depth } => run_count_positions(depth),
        Chess::Play { depth } => play_computer(depth),
        Chess::Watch => computer_vs_computer(),
        Chess::Pvp => player_vs_player(),
        Chess::FindMagic => run_find_magic(),
    }
}

fn run_count_positions(depth: u8) {
    let depths = 0..=depth;
    let mut ray_table = RayTable::new();
    ray_table.populate();

    for depth in depths {
        let mut board = Board::starting_position();
        let count = count_positions(depth, &mut board, &ray_table, Color::White);

        println!("depth: {}, positions: {}", depth, count);
    }
}
