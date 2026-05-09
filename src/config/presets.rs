use crate::config::model::RepoPilotConfig;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    /// Tighter thresholds — catches more issues. Good for green-field projects.
    Strict,
    /// Factory defaults. Suitable for most projects.
    Balanced,
    /// Relaxed thresholds — fewer findings. Useful when adopting RepoPilot on legacy code.
    Lenient,
}

use std::str::FromStr;

impl FromStr for Preset {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "strict" => Ok(Self::Strict),
            "balanced" => Ok(Self::Balanced),
            "lenient" => Ok(Self::Lenient),
            _ => Err(()),
        }
    }
}

/// Overrides config thresholds according to the chosen preset.
/// CLI flags applied afterwards still take precedence.
pub fn apply_preset(config: &mut RepoPilotConfig, preset: Preset) {
    match preset {
        Preset::Balanced => {} // defaults — nothing to change
        Preset::Strict => {
            config.architecture.max_file_lines = 200;
            config.architecture.huge_file_lines = 500;
            config.architecture.max_directory_modules = 15;
            config.architecture.max_directory_depth = 4;
            config.architecture.max_function_lines = 40;
            config.architecture.max_fan_out = 10;
            config.code_quality.complexity_medium_threshold = 150;
            config.code_quality.complexity_high_threshold = 300;
        }
        Preset::Lenient => {
            config.architecture.max_file_lines = 600;
            config.architecture.huge_file_lines = 2000;
            config.architecture.max_directory_modules = 35;
            config.architecture.max_directory_depth = 8;
            config.architecture.max_function_lines = 100;
            config.architecture.max_fan_out = 25;
            config.code_quality.complexity_medium_threshold = 400;
            config.code_quality.complexity_high_threshold = 800;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::RepoPilotConfig;

    #[test]
    fn strict_lowers_thresholds() {
        let mut cfg = RepoPilotConfig::default();
        let balanced_file_lines = cfg.architecture.max_file_lines;
        apply_preset(&mut cfg, Preset::Strict);
        assert!(cfg.architecture.max_file_lines < balanced_file_lines);
        assert!(cfg.architecture.max_function_lines < 50);
    }

    #[test]
    fn lenient_raises_thresholds() {
        let mut cfg = RepoPilotConfig::default();
        let balanced_file_lines = cfg.architecture.max_file_lines;
        apply_preset(&mut cfg, Preset::Lenient);
        assert!(cfg.architecture.max_file_lines > balanced_file_lines);
        assert!(cfg.architecture.max_function_lines > 50);
    }

    #[test]
    fn balanced_keeps_defaults() {
        let default = RepoPilotConfig::default();
        let mut cfg = RepoPilotConfig::default();
        apply_preset(&mut cfg, Preset::Balanced);
        assert_eq!(
            cfg.architecture.max_file_lines,
            default.architecture.max_file_lines
        );
    }

    #[test]
    fn from_str_parses_all_variants() {
        assert_eq!("strict".parse(), Ok(Preset::Strict));
        assert_eq!("balanced".parse(), Ok(Preset::Balanced));
        assert_eq!("lenient".parse(), Ok(Preset::Lenient));
        assert_eq!("unknown".parse::<Preset>(), Err(()));
    }
}
