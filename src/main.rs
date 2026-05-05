mod app;
mod cli;

use clap::Parser;
use cli::Cli;

fn main() {
    let cli = Cli::parse();

    if let Err(error) = app::run(cli) {
        eprintln!("RepoPilot failed: {error}");
        std::process::exit(1);
    }
}
