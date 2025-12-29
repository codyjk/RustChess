//! Chess engine binary entry point.

mod cli;

use cli::{commands::Command, Chess};
use structopt::StructOpt;

fn main() {
    env_logger::init();
    Chess::from_args().execute();
}
