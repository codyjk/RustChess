//! Determine Stockfish ELO command - measure engine strength.

use chess::game::stockfish_elo::determine_stockfish_elo;
use structopt::StructOpt;

use super::Command;

#[derive(StructOpt)]
pub struct DetermineStockfishEloArgs {
    #[structopt(short, long, default_value = "4")]
    pub depth: u8,
    #[structopt(short, long, default_value = "1000")]
    pub starting_elo: u32,
}

impl Command for DetermineStockfishEloArgs {
    fn execute(self) {
        determine_stockfish_elo(self.depth, self.starting_elo);
    }
}
