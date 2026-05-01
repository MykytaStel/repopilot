mod scan;

use clap::{Parser, Subcommand};
use scan::scanner::scan_path;
use scan::types::ScanSummary;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "repopilot")]
#[command(about = "A CLI tool for analyzing codebases", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Scan { path: PathBuf },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan { path } => match scan_path(&path) {
            Ok(summary) => {
                println!("RepoPilot Scan");
                println!("Path: {}", path.display());
                print_summary(&summary);
            }
            Err(error) => {
                eprintln!("Failed to scan path: {error}");
                std::process::exit(1);
            }
        },
    }
}

fn print_summary(summary: &ScanSummary) {
    println!("Files analyzed: {}", summary.files_count);
    println!("Directories analyzed: {}", summary.directories_count);
}
