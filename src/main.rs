mod scan;

use clap::{Parser, Subcommand};
use scan::scanner::scan_path;
use scan::types::ScanSummary;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "repopilot")]
#[command(about = "Local-first codebase audit CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan a project, folder, or file
    Scan {
        /// Path to project, folder, or file
        path: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan { path } => match scan_path(&path) {
            Ok(summary) => {
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
    println!("RepoPilot Scan");
    println!("Path: {}", summary.root_path.display());
    println!();

    println!("Files analyzed: {}", summary.files_count);
    println!("Directories analyzed: {}", summary.directories_count);
    println!("Lines of code: {}", summary.lines_of_code);
    println!();

    println!("Languages:");
    if summary.languages.is_empty() {
        println!("  No languages detected");
    } else {
        for language in &summary.languages {
            println!("  {}: {} files", language.name, language.files_count);
        }
    }

    println!();
    println!("Code markers:");
    if summary.markers.is_empty() {
        println!("  No TODO/FIXME/HACK markers found");
    } else {
        for marker in &summary.markers {
            println!(
                "  [{}] {}:{} — {}",
                marker.kind,
                marker.path.display(),
                marker.line_number,
                marker.text.trim()
            );
        }
    }
}
