//! Determine Stockfish ELO command - measure engine strength.

use chess::game::stockfish_elo::determine_stockfish_elo;
use structopt::StructOpt;

use super::Command;

#[derive(StructOpt)]
pub struct DetermineStockfishEloArgs {
    #[structopt(short, long, default_value = "4")]
    pub depth: u8,
    #[structopt(short, long, default_value = "1320")]
    pub starting_elo: u32,
    #[structopt(long, help = "Disable TUI and print progress to stdout")]
    pub no_tui: bool,
    #[structopt(
        short,
        long,
        default_value = "1000",
        help = "Time limit per move in ms"
    )]
    pub time_limit: u64,
}

impl Command for DetermineStockfishEloArgs {
    fn execute(self) {
        determine_stockfish_elo(self.depth, self.starting_elo, self.no_tui, self.time_limit);
    }
}
