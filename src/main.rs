use clap::Parser;

use crate::cli::Cli;

mod cli;

fn main() {
    let _cli = Cli::parse();
}
