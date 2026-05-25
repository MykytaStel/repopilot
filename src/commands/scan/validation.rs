use crate::cli::ScanOptions;
use crate::commands::product_scan::ProductScanMode;
use crate::commands::{CliExit, EXIT_USAGE};
use repopilot::findings::visibility::FindingVisibilityProfile;

pub(super) fn validate_scan_options(
    options: &ScanOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    if options.fail_on.is_some() && options.fail_on_priority.is_some() {
        return Err(Box::new(CliExit {
            code: EXIT_USAGE,
            message: "`--fail-on` and `--fail-on-priority` cannot be used together".to_string(),
        }));
    }

    if options.changed && options.since.is_some() {
        return Err(Box::new(CliExit {
            code: EXIT_USAGE,
            message: "`--changed` and `--since` cannot be used together".to_string(),
        }));
    }

    if options.workspace && (options.changed || options.since.is_some()) {
        return Err(Box::new(CliExit {
            code: EXIT_USAGE,
            message: "`--workspace` cannot be used with changed scans".to_string(),
        }));
    }

    if options.no_color && matches!(options.color, Some(crate::cli::ColorArg::Always)) {
        return Err(Box::new(CliExit {
            code: EXIT_USAGE,
            message: "`--color always` cannot be used with `--no-color`".to_string(),
        }));
    }

    Ok(())
}

pub(super) fn scan_mode_from_options(options: &ScanOptions) -> ProductScanMode {
    if options.changed || options.since.is_some() {
        ProductScanMode::Changed {
            since: options.since.clone(),
        }
    } else if options.workspace {
        ProductScanMode::Workspace
    } else {
        ProductScanMode::Full
    }
}

pub(super) fn scan_visibility_profile(options: &ScanOptions) -> FindingVisibilityProfile {
    if options.include_maintainability || !options.rule.is_empty() {
        return FindingVisibilityProfile::Strict;
    }

    match options.profile {
        Some(crate::cli::ScanProfileArg::Strict) => FindingVisibilityProfile::Strict,
        Some(crate::cli::ScanProfileArg::Default) | None => FindingVisibilityProfile::Default,
    }
}
