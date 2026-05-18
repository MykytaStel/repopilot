use crate::cli::CacheCommands;
use repopilot::scan::cache::{cache_dir, clear_cache};

pub fn run(command: CacheCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        CacheCommands::Clear(options) => {
            let cache_path = cache_dir(&options.path);
            clear_cache(&options.path)?;
            println!("Cache cleared: {}", cache_path.display());
            Ok(())
        }
    }
}
