use crate::commands::{CliExit, EXIT_USAGE};
use repopilot::output::vibe::VibeCategory;

pub const VALID_FOCUS_VALUES: &str = "security, arch, architecture, quality, framework, all";

pub fn parse_focus_category(
    focus: Option<&str>,
) -> Result<Option<VibeCategory>, Box<dyn std::error::Error>> {
    match focus {
        Some(value) => Ok(Some(value.parse::<VibeCategory>().map_err(|_| {
            CliExit {
                code: EXIT_USAGE,
                message: format!("Invalid focus '{value}'. Expected: {VALID_FOCUS_VALUES}"),
            }
        })?)),
        None => Ok(None),
    }
}
