//! UCI (Universal Chess Interface) command implementation

use chess::uci::UciProtocol;

use super::Command;

/// UCI protocol mode - starts UCI interface for external chess GUIs
#[derive(structopt::StructOpt)]
pub struct UciArgs {
    // No arguments needed for UCI mode
}

impl Command for UciArgs {
    fn execute(self) {
        let mut protocol = UciProtocol::new();
        protocol.run();
    }
}
