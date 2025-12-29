use std::str::FromStr;
use std::time::{Duration, SystemTime};

use crate::alpha_beta_searcher::SearchContext;
use crate::board::color::Color;
use crate::board::Board;
use crate::chess_search::search_best_move;
use crate::move_generator::MoveGenerator;

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
    let depths = 1..=depth;
    let move_generator = MoveGenerator::default();

    let mut total_positions = 0;
    let mut total_duration = Duration::from_secs(0);

    for depth in depths {
        let mut board = Board::default();

        let starting_time = SystemTime::now();
        let count = match strategy {
            CountPositionsStrategy::All => {
                move_generator.count_positions(depth, &mut board, Color::White)
            }
            CountPositionsStrategy::AlphaBeta => {
                let mut search_context = SearchContext::new(depth);
                search_best_move(&mut search_context, &mut board).unwrap();
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
