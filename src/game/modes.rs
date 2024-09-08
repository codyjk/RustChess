use super::{Game, GameEnding};
use crate::alpha_beta_searcher::{alpha_beta_search, SearchContext};
use crate::board::color::Color;
use crate::board::Board;
use crate::chess_move::algebraic_notation::enumerate_candidate_moves_with_algebraic_notation;
use crate::chess_move::ChessMove;
use crate::game::command::{Command, MakeWaterfallMove};
use crate::input_handler;
use crate::move_generator::MoveGenerator;
use common::bitboard::square::from_rank_file;
use std::str::FromStr;
use std::thread::sleep;
use std::time::{Duration, SystemTime};
use termion::{clear, cursor};

pub fn play_computer(depth: u8, player_color: Color) {
    let game = &mut Game::new(depth);

    print!("{}{}", cursor::Goto(1, 1), clear::All);
    println!("You are {}", player_color);
    print_board(&game.board);
    print_enter_move_prompt();

    loop {
        match game.check_game_over_for_current_turn() {
            Some(GameEnding::Checkmate) => {
                println!("checkmate!");
                break;
            }
            Some(GameEnding::Stalemate) => {
                println!("stalemate!");
                break;
            }
            _ => (),
        };

        // Precalculate the moves and their algebraic notations, so that we
        // can render it after a move is made.
        let enumerated_candidate_moves = enumerated_candidate_moves(game);

        let command: Box<dyn Command> = if player_color == game.board.turn() {
            match input_handler::parse_player_move_input() {
                Ok(command) => command,
                Err(msg) => {
                    println!("{}", msg);
                    continue;
                }
            }
        } else {
            Box::<MakeWaterfallMove>::default()
        };

        let start_time = SystemTime::now();
        match command.execute(game) {
            Ok(_chess_move) => {
                let duration = SystemTime::now().duration_since(start_time).unwrap();
                print!("{}{}", cursor::Goto(1, 1), clear::All);
                game.board.toggle_turn();

                print_board_and_stats(game, enumerated_candidate_moves);
                if player_color == game.board.turn() {
                    println!("* Move took: {:?}", duration);
                    print_enter_move_prompt();
                }
                continue;
            }
            Err(error) => println!("error: {}", error),
        }
    }
}

pub fn computer_vs_computer(move_limit: u8, sleep_between_turns_in_ms: u64, depth: u8) {
    let mut game = Game::new(depth);

    println!("{}", clear::All);

    loop {
        sleep(Duration::from_millis(sleep_between_turns_in_ms));

        match game.check_game_over_for_current_turn() {
            Some(GameEnding::Checkmate) => {
                println!("checkmate!");
                break;
            }
            Some(GameEnding::Stalemate) => {
                println!("stalemate!");
                break;
            }
            Some(GameEnding::Draw) => {
                println!("draw!");
                break;
            }
            _ => (),
        };

        if move_limit > 0 && game.fullmove_clock() > move_limit {
            break;
        }

        // Precalculate the moves and their algebraic notations, so that we
        // can render it after a move is made.
        let enumerated_candidate_moves = enumerated_candidate_moves(&mut game);

        let result = game.make_waterfall_book_then_alpha_beta_move();

        match result {
            Ok(_chess_move) => {
                println!("{}", clear::All);
                game.board.toggle_turn();
                print_board_and_stats(&mut game, enumerated_candidate_moves);
                game.reset_move_generator_cache_hit_count();
                continue;
            }
            Err(error) => {
                println!("error: {}", error);
                break;
            }
        }
    }
}

pub fn player_vs_player() {
    let game = &mut Game::new(0);
    loop {
        println!("turn: {}", game.board.turn());
        println!("{}", game.board);

        match game.check_game_over_for_current_turn() {
            Some(GameEnding::Checkmate) => {
                println!("checkmate!");
                break;
            }
            Some(GameEnding::Stalemate) => {
                println!("stalemate!");
                break;
            }
            Some(GameEnding::Draw) => {
                println!("draw!");
                break;
            }
            _ => (),
        };

        let command = match input_handler::parse_player_move_input() {
            Ok(command) => command,
            Err(msg) => {
                println!("{}", msg);
                continue;
            }
        };

        match command.execute(game) {
            Ok(_chess_move) => {
                game.board.toggle_turn();
                continue;
            }
            Err(error) => println!("error: {}", error),
        }
    }
}

#[derive(Debug)]
pub enum CountPositionsStrategy {
    All,
    AlphaBeta,
}

impl FromStr for CountPositionsStrategy {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "all" => Ok(CountPositionsStrategy::All),
            "alpha-beta" => Ok(CountPositionsStrategy::AlphaBeta),
            _ => Err("invalid strategy; options are: all, alpha-beta"),
        }
    }
}

pub fn run_count_positions(depth: u8, strategy: CountPositionsStrategy) {
    let depths = 0..=depth;
    let mut move_generator = MoveGenerator::new();

    let mut total_positions = 0;
    let mut total_duration = Duration::from_secs(0);

    for depth in depths {
        let mut board = Board::starting_position();

        let starting_time = SystemTime::now();
        let count = match strategy {
            CountPositionsStrategy::All => {
                move_generator.count_positions(depth, &mut board, Color::White)
            }
            CountPositionsStrategy::AlphaBeta => {
                let mut search_context = SearchContext::new(depth);
                alpha_beta_search(&mut search_context, &mut board, &mut move_generator).unwrap();
                search_context.searched_position_count()
            }
        };
        let duration = SystemTime::now().duration_since(starting_time).unwrap();
        let positions_per_second = count as f64 / duration.as_secs_f64();

        total_positions += count;
        total_duration += duration;

        println!(
            "depth: {}, positions: {}, positions per second: {}",
            depth, count, positions_per_second
        );
    }

    println!(
        "total positions: {}, total duration: {:?}, positions per second: {}",
        total_positions,
        total_duration,
        total_positions as f64 / total_duration.as_secs_f64()
    );
}

fn enumerated_candidate_moves(game: &mut Game) -> Vec<(ChessMove, String)> {
    let board = &mut game.board;
    let current_turn = board.turn();
    let move_generator = &mut game.move_generator;
    enumerate_candidate_moves_with_algebraic_notation(board, current_turn, move_generator)
}

fn print_board_and_stats(game: &mut Game, enumerated_candidate_moves: Vec<(ChessMove, String)>) {
    let last_move_algebraic = match game.last_move() {
        Some(chess_move) => enumerated_candidate_moves
            .iter()
            .find(|(move_, _)| move_ == &chess_move)
            .unwrap()
            .1
            .clone(),
        None => "-".to_string(),
    };
    let searched_position_count = game.searched_position_count();
    let searched_position_message = match searched_position_count {
        0 => "0 (book move)".to_string(),
        _ => format!(
            "{} (depth {})",
            searched_position_count,
            game.search_depth()
        ),
    };
    println!(
        "{} chose move: {}\n",
        game.board.turn().opposite(),
        last_move_algebraic
    );
    print_board(&game.board);
    println!("* Turn: {}", game.board.turn());
    println!("* Halfmove clock: {}", game.board.halfmove_clock());
    println!("* Score: {}", game.score(game.board.turn()));
    println!("* Positions searched: {}", searched_position_message);
}

fn print_enter_move_prompt() {
    println!("Enter your move:");
}

fn print_board(board: &Board) {
    let mut board_str = String::new();
    board_str.push_str("    a   b   c   d   e   f   g   h\n");
    board_str.push_str("  ┌───┬───┬───┬───┬───┬───┬───┬───┐\n");
    for rank in 0..8 {
        let transposed_rank = 7 - rank;
        board_str.push_str(&format!("{} │", transposed_rank + 1));
        for file in 0..8 {
            let square = from_rank_file(transposed_rank, file);
            let piece = board.get(square);
            let piece_str = match piece {
                Some((piece, color)) => piece.to_unicode_piece_char(color).to_string(),
                None => if (rank + file) % 2 == 0 { " " } else { "·" }.to_string(),
            };
            board_str.push_str(&format!(" {} │", piece_str));
        }
        board_str.push_str(&format!(" {}\n", transposed_rank + 1));
        if rank < 7 {
            board_str.push_str("  ├───┼───┼───┼───┼───┼───┼───┼───┤\n");
        } else {
            board_str.push_str("  └───┴───┴───┴───┴───┴───┴───┴───┘\n");
        }
    }
    board_str.push_str("    a   b   c   d   e   f   g   h\n");
    println!("{}", board_str);
}
