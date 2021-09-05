use chess::board::color::Color;
use chess::board::Board;
use chess::game::command::MakeMinimaxOptimalMove;
use chess::game::Game;
use chess::input_handler;
use chess::moves;
use chess::moves::ray_table::RayTable;
use structopt::StructOpt;
use termion::clear;

#[derive(StructOpt, Debug)]
#[structopt(name = "chess", about = "chess engine cli")]
enum Chess {
    CountPositions {
        #[structopt(short, long, default_value = "3")]
        depth: u8,
    },
    Play,
}

fn main() {
    let args = Chess::from_args();
    match args {
        Chess::CountPositions { depth } => run_count_positions(depth),
        Chess::Play => play_computer(),
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

fn count_positions(depth: u8, board: &mut Board, ray_table: &RayTable, color: Color) -> usize {
    let candidates = moves::generate(board, color, ray_table);
    let mut count = candidates.len();

    if depth == 0 {
        return count;
    }

    let next_color = color.opposite();

    for chessmove in candidates {
        board.apply(chessmove).unwrap();
        count += count_positions(depth - 1, board, ray_table, next_color);
        board.undo(chessmove).unwrap();
    }

    count
}

fn play_computer() {
    let game = &mut Game::new();
    let rand: u8 = rand::random();
    let player_color = match rand % 2 {
        0 => Color::White,
        _ => Color::Black,
    };
    println!("{}", clear::All);
    loop {
        println!("{}", game.render_board());
        let command = if player_color == game.turn() {
            match input_handler::parse_command() {
                Ok(command) => command,
                Err(msg) => {
                    println!("{}", msg);
                    continue;
                }
            }
        } else {
            Box::new(MakeMinimaxOptimalMove {})
        };

        match command.execute(game) {
            Ok(chessmove) => {
                println!("{}", clear::All);
                let player = match player_color {
                    c if c == game.turn() => "you",
                    _ => "computer",
                };
                println!("{} chose {}", player, chessmove);
                game.next_turn();
                continue;
            }
            Err(error) => println!("error: {}", error),
        }
    }
}
