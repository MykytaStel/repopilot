mod app;
mod cli;

use clap::Parser;
use cli::Cli;

fn main() {
    let cli = Cli::parse();

    if let Err(error) = app::run(cli) {
        if let Some(exit) = error.downcast_ref::<app::CliExit>() {
            eprintln!("{exit}");
            std::process::exit(exit.code);
        }

        eprintln!("RepoPilot failed: {error}");
        std::process::exit(1);
    }
}
