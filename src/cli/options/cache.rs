use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args)]
pub struct CacheOptions {
    #[command(subcommand)]
    pub command: CacheCommands,
}

#[derive(Subcommand)]
pub enum CacheCommands {
    /// Clear RepoPilot's local scan cache
    #[command(
        about = "Clear RepoPilot's local scan cache",
        long_about = "Removes only the .repopilot/cache directory for the selected path.\n\n\
The command succeeds when the cache directory does not exist.",
        after_help = "EXAMPLES:\n  \
repopilot cache clear\n  \
repopilot cache clear ."
    )]
    Clear(CacheClearOptions),
}

#[derive(Args)]
pub struct CacheClearOptions {
    /// Repository or project path whose .repopilot/cache directory should be removed
    #[arg(default_value = ".")]
    pub path: PathBuf,
}
