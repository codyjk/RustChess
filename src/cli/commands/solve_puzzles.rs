//! Solve-puzzles command -- run the puzzle suite once and report results.

use structopt::StructOpt;

use super::Command;

#[derive(StructOpt)]
pub struct SolvePuzzlesArgs {
    /// Only run puzzles for a specific tier (1=tactical, 2=strategic, 3=deep)
    #[structopt(short, long)]
    pub tier: Option<u8>,
}

impl Command for SolvePuzzlesArgs {
    fn execute(self) {
        chess::game::puzzle_suite::run_puzzle_suite(self.tier);
    }
}
