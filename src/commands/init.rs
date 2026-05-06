use repopilot::config::template::default_config_toml;
use std::fs;
use std::path::PathBuf;

pub fn run(force: bool, path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if path.exists() && !force {
        println!(
            "Config already exists at {}. Use `repopilot init --force` to overwrite it.",
            path.display()
        );
        return Ok(());
    }

    fs::write(&path, default_config_toml())?;
    println!("Created RepoPilot config at {}", path.display());

    Ok(())
}
