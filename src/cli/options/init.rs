use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct InitOptions {
    /// Overwrite an existing config file
    #[arg(long)]
    pub force: bool,

    /// Config file path to write
    #[arg(long, default_value = "repopilot.toml")]
    pub path: PathBuf,
}
