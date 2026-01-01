//! Chess engine binary entry point.

mod cli;
#[cfg(feature = "instrumentation")]
mod instrumentation;

use cli::{commands::Command, Chess};
use structopt::StructOpt;

fn main() {
    #[cfg(feature = "instrumentation")]
    instrumentation::init_tracing();
    #[cfg(not(feature = "instrumentation"))]
    env_logger::init();

    Chess::from_args().execute();

    #[cfg(feature = "instrumentation")]
    instrumentation::print_timing_statistics();
}
