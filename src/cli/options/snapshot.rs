use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct SnapshotOptions {
    /// Path inside the repository to snapshot (defaults to the current directory)
    #[arg(default_value = ".")]
    pub path: PathBuf,
}
