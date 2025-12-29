//! Count positions command - count possible positions at a given depth.

use chess::game::position_counter::{run_count_positions, CountPositionsStrategy};
use structopt::StructOpt;

use super::Command;

#[derive(StructOpt)]
pub struct CountPositionsArgs {
    #[structopt(short, long, default_value = "4")]
    pub depth: u8,
    #[structopt(short, long, default_value = "all")]
    pub strategy: CountPositionsStrategy,
}

impl Command for CountPositionsArgs {
    fn execute(self) {
        run_count_positions(self.depth, self.strategy);
    }
}
