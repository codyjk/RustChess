//! Benchmark alpha-beta command - quick performance testing.

use chess::game::alpha_beta_benchmark::{list_positions, run_alpha_beta_benchmark};
use structopt::StructOpt;

use super::Command;

#[derive(StructOpt)]
pub struct BenchmarkAlphaBetaArgs {
    #[structopt(short, long, default_value = "4")]
    pub depth: u8,
    #[structopt(short, long)]
    pub parallel: bool,
    #[structopt(long)]
    pub position: Option<String>,
    #[structopt(long)]
    pub list: bool,
}

impl Command for BenchmarkAlphaBetaArgs {
    fn execute(self) {
        if self.list {
            println!("Available benchmark positions:");
            list_positions();
            return;
        }
        run_alpha_beta_benchmark(self.depth, self.parallel, self.position);
    }
}
