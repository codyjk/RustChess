use std::io;

use chess::board::color::Color;
use chess::board::square;
use chess::board::Board;
use chess::moves::generate;
use chess::moves::ray_table::RayTable;
use regex::Regex;

fn main() {
    let mut board = Board::starting_position();
    let mut ray_table = RayTable::new();
    ray_table.populate();

    loop {
        println!("{}", board.to_ascii());
        let mut input = String::new();

        let parsed = match io::stdin().read_line(&mut input) {
            Ok(_n) => MoveCommand::parse(&input.trim_start().trim_end()),
            Err(error) => {
                println!("error: {}", error);
                continue;
            }
        };

        let partial_move = match parsed {
            Ok(move_command) => move_command,
            Err(error) => {
                println!("invalid move: {}", error);
                continue;
            }
        };

        let mut moves = generate(&board, Color::White, &ray_table);
        moves.append(&mut generate(&board, Color::Black, &ray_table));

        let maybe_chessmove = moves.iter().find(|&m| {
            m.from_square == partial_move.from_square && m.to_square == partial_move.to_square
        });

        let chessmove = match maybe_chessmove {
            Some(result) => result,
            None => {
                println!("invalid move");
                continue;
            }
        };

        let result = board.apply(*chessmove);
        let captured_piece = match result {
            Ok(piece) => piece,
            Err(error) => {
                println!("move error: {}", error);
                continue;
            }
        };

        match captured_piece {
            Some((piece, color)) => println!(
                "captured {} on {}",
                piece.to_fen(color),
                square::to_algebraic(chessmove.to_square),
            ),
            _ => (),
        };
    }
}

struct MoveCommand {
    from_square: u64,
    to_square: u64,
}

impl MoveCommand {
    pub fn parse(command: &str) -> Result<MoveCommand, &'static str> {
        let re = Regex::new("^([a-h][1-8])([a-h][1-8])$").unwrap();
        let caps = match re.captures(&command) {
            Some(captures) => captures,
            None => return Err("invalid move"),
        };

        Ok(Self {
            from_square: square::from_algebraic(&caps[1]),
            to_square: square::from_algebraic(&caps[2]),
        })
    }
}
